//! Provides abstractions for the ethernet device.

pub use init::PhyError;

use alloc::boxed::Box;
use alloc::vec::Vec;

use stm32f7::stm32f7x6::{ETHERNET_DMA, ETHERNET_MAC, RCC, SYSCFG};
use volatile::Volatile;

use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder, Routes};
use smoltcp::phy::{Device, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpCidr, Ipv4Address};

mod init;
mod phy;
mod rx;
mod tx;

/// The maximum transmission unit.
pub const MTU: usize = 1536;

/// Represents an ethernet device that allows sending and receiving packets.
///
/// This struct implements the [smoltcp::phy::Device] trait.
pub struct EthernetDevice<'d> {
    rx: RxDevice,
    tx: TxDevice,
    ethernet_dma: &'d mut ETHERNET_DMA,
    ethernet_address: EthernetAddress,
}

impl<'d> EthernetDevice<'d> {
    /// Creates and initializes a new `EthernetDevice`.
    ///
    /// This function takes the following parameters:
    ///
    /// - `rx_config` and `tx_config` for configuring the receive and transmission buffers.
    /// - References to the `RCC`, `SYSCFG` and `ETHERNET_MAC` register blocks used for
    ///   initializing the device.
    /// - A reference to the `ETHERNET_DMA` register. This reference determines the lifetime
    ///   of the resulting EthernetDevice.
    /// - The `EthernetAddress` that should be used for the interface.
    pub fn new(
        rx_config: RxConfig,
        tx_config: TxConfig,
        rcc: &mut RCC,
        syscfg: &mut SYSCFG,
        ethernet_mac: &mut ETHERNET_MAC,
        ethernet_dma: &'d mut ETHERNET_DMA,
        ethernet_address: EthernetAddress,
    ) -> Result<Self, PhyError> {
        use byteorder::{ByteOrder, LittleEndian};

        init::init(rcc, syscfg, ethernet_mac, ethernet_dma)?;

        let rx_device = RxDevice::new(rx_config)?;
        let tx_device = TxDevice::new(tx_config);

        ethernet_dma.dmardlar.write(|w| {
            w.srl()
                .bits(&rx_device.descriptors[0] as *const Volatile<_> as u32)
        });
        ethernet_dma.dmatdlar.write(|w| {
            w.stl()
                .bits(tx_device.front_of_queue() as *const Volatile<_> as u32)
        });

        let eth_bytes = ethernet_address.as_bytes();
        ethernet_mac
            .maca0lr
            .write(|w| w.maca0l().bits(LittleEndian::read_u32(&eth_bytes[..4])));
        ethernet_mac
            .maca0hr
            .write(|w| w.maca0h().bits(LittleEndian::read_u16(&eth_bytes[4..])));

        init::start(ethernet_mac, ethernet_dma);
        Ok(EthernetDevice {
            rx: rx_device,
            tx: tx_device,
            ethernet_dma: ethernet_dma,
            ethernet_address: ethernet_address,
        })
    }

    /// Transforms the ethernet device into a smoltcp ethernet network interface.
    pub fn into_interface<'a>(self) -> EthernetInterface<'a, 'a, 'a, Self> {
        use alloc::collections::BTreeMap;
        use smoltcp::iface::NeighborCache;

        let ip_addrs = [IpCidr::new(Ipv4Address::UNSPECIFIED.into(), 0)];
        let routes_storage = Box::leak(Box::new([None; 1]));
        let routes = Routes::new(&mut routes_storage[..]);

        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        let ethernet_address = self.ethernet_address;
        let interface_builder = EthernetInterfaceBuilder::new(self);
        let interface_builder = interface_builder.ethernet_addr(ethernet_address);
        let interface_builder = interface_builder.ip_addrs(ip_addrs);
        let interface_builder = interface_builder.routes(routes);
        let interface_builder = interface_builder.neighbor_cache(neighbor_cache);
        interface_builder.finalize()
    }
}

impl<'d> Drop for EthernetDevice<'d> {
    fn drop(&mut self) {
        // TODO stop ethernet device and wait for idle
        unimplemented!();
    }
}

impl<'a, 'd> Device<'a> for EthernetDevice<'d> {
    type RxToken = RxToken<'a>;
    type TxToken = TxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        if !self.rx.new_data_received() {
            return None;
        }
        let rx = RxToken { rx: &mut self.rx };
        let tx = TxToken {
            tx: &mut self.tx,
            ethernet_dma: &mut self.ethernet_dma,
        };
        Some((rx, tx))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        if !self.tx.descriptor_available() {
            return None;
        }
        Some(TxToken {
            tx: &mut self.tx,
            ethernet_dma: &mut self.ethernet_dma,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.max_transmission_unit = MTU;
        capabilities
    }
}

/// A token that allows receiving a single packet.
///
/// Used in the [Device] trait.
pub struct RxToken<'a> {
    rx: &'a mut RxDevice,
}

impl<'a> ::smoltcp::phy::RxToken for RxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> ::smoltcp::Result<R>
    where
        F: FnOnce(&[u8]) -> ::smoltcp::Result<R>,
    {
        self.rx.receive(f).map_err(|err| match err {
            ReceiveError::Processing(e) => e,
            _ => ::smoltcp::Error::Truncated,
        })
    }
}

/// A token that allows tranmitting a single packet.
///
/// Used in the [Device] trait.
pub struct TxToken<'a> {
    tx: &'a mut TxDevice,
    ethernet_dma: &'a mut ETHERNET_DMA,
}

impl<'a> ::smoltcp::phy::TxToken for TxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> ::smoltcp::Result<R>
    where
        F: FnOnce(&mut [u8]) -> ::smoltcp::Result<R>,
    {
        let mut data = vec![0; len].into_boxed_slice();
        let ret = f(&mut data)?;
        self.tx.insert(data);
        self.start_send();
        Ok(ret)
    }
}

impl<'a> TxToken<'a> {
    fn start_send(&mut self) {
        // read transmit process state
        let state = self.ethernet_dma.dmasr.read().tps();
        if state.is_stopped() {
            panic!("stopped")
        } else if state.is_suspended() {
            // write poll demand register
            self.ethernet_dma.dmatpdr.write(|w| w.tpd().poll());
        } else if state.is_running()
            || state.is_running_fetching()
            || state.is_running_waiting()
            || state.is_running_reading()
        {
            // do nothing
        } else {
            panic!("unexpected transmit process state");
        }
    }
}

/// An error that occurred while receiving a packet.
#[derive(Debug, PartialEq, Eq)]
pub enum ReceiveError {
    /// A cyclic redundancy check (CRC) error occurred on the received frame.
    Crc,
    /// The RX_ERR signal was asserted while RX_DV was asserted during frame reception.
    Receive,
    /// The Receive watchdog timer has expired while receiving the current frame.
    WatchdogTimeout,
    /// A late collision has occurred while receiving the frame in half-duplex mode.
    LateCollision,
    /// This error depends on the configuration and can also be a IPv4/IPv6 header checksum
    /// error. TODO: Figure out which error variant we use.
    GiantFrame,
    /// The received frame was damaged due to buffer overflow.
    Overflow,
    /// Indicates a frame truncation caused by a frame that does not fit within the current
    /// descriptor buffers
    Descriptor,
    /// An error occurred while processing the packet.
    Processing(::smoltcp::Error),
}

struct RxDevice {
    config: RxConfig,
    buffer: Box<[u8]>,
    descriptors: Box<[Volatile<rx::RxDescriptor>]>,
    next_descriptor: usize,
}

impl RxDevice {
    fn new(config: RxConfig) -> Result<RxDevice, PhyError> {
        use self::rx::RxDescriptor;

        let buffer = vec![0; config.buffer_size].into_boxed_slice();
        let descriptor_num = config.number_of_descriptors;
        let mut descriptors = Vec::with_capacity(descriptor_num);

        for i in 0..descriptor_num {
            let buffer_offset = config.descriptor_buffer_offset(i);
            let buffer_start = &buffer[buffer_offset];
            let buffer_size = config.descriptor_buffer_size(i);

            let mut descriptor = RxDescriptor::new(buffer_start, buffer_size);
            if i == descriptor_num - 1 {
                descriptor.set_end_of_ring(true);
            }
            descriptors.push(Volatile::new(descriptor));
        }

        Ok(RxDevice {
            config: config,
            buffer: buffer,
            descriptors: descriptors.into_boxed_slice(),
            next_descriptor: 0,
        })
    }

    fn new_data_received(&self) -> bool {
        let descriptor = self.descriptors[self.next_descriptor].read();
        !descriptor.own() && descriptor.is_first_descriptor()
    }

    fn receive<T, F>(&mut self, f: F) -> Result<T, ReceiveError>
    where
        F: FnOnce(&[u8]) -> ::smoltcp::Result<T>,
    {
        let descriptor_index = self.next_descriptor;
        let descriptor = self.descriptors[descriptor_index].read();

        if descriptor.own() || !descriptor.is_first_descriptor() {
            return Err(ReceiveError::Processing(::smoltcp::Error::Exhausted));
        }
        if let rx::ChecksumResult::Error(_, _) = descriptor.checksum_result() {
            return Err(ReceiveError::Processing(::smoltcp::Error::Checksum));
        }

        // find the last descriptor belonging to the received packet
        let mut last_descriptor = descriptor;
        let mut i = 0;
        while !last_descriptor.is_last_descriptor() {
            i += 1;
            // Descriptors wrap around, but we don't want packets to wrap around. So we require
            // that the last descriptor in the list is large enough to hold all received packets.
            // This assertion checks that no wraparound occurs.
            assert!(
                descriptor_index + i < self.descriptors.len(),
                "buffer of last descriptor in \
                 list must be large enough to hold received packets without wrap-around"
            );
            last_descriptor = self.descriptors[descriptor_index + i].read();
            if last_descriptor.own() {
                return Err(ReceiveError::Processing(::smoltcp::Error::Exhausted)); // packet is not fully received
            }
        }

        // check for errors
        let mut error = None;
        if last_descriptor.error() {
            if last_descriptor.crc_error() {
                error = Some(ReceiveError::Crc);
            }
            if last_descriptor.receive_error() {
                error = Some(ReceiveError::Receive);
            }
            if last_descriptor.watchdog_timeout_error() {
                error = Some(ReceiveError::WatchdogTimeout);
            }
            if last_descriptor.late_collision_error() {
                error = Some(ReceiveError::LateCollision);
            }
            if last_descriptor.giant_frame_error() {
                error = Some(ReceiveError::GiantFrame);
            }
            if last_descriptor.overflow_error() {
                error = Some(ReceiveError::Overflow);
            }
            if last_descriptor.descriptor_error() {
                error = Some(ReceiveError::Descriptor);
            }
        }

        let ret = match error {
            Some(error) => Err(error),
            None => {
                // read data and pass it to processing function
                let offset = self.config.descriptor_buffer_offset(descriptor_index);
                let len = last_descriptor.frame_len();
                let data = &self.buffer[offset..(offset + len)];
                f(data).map_err(ReceiveError::Processing)
            }
        };

        // reset descriptor(s) and update next_descriptor
        let mut next = descriptor_index;
        loop {
            let descriptor = self.descriptors[next].read();
            self.descriptors[next].update(|d| d.reset());
            next = (next + 1) % self.descriptors.len();
            if descriptor.is_last_descriptor() {
                break;
            }
        }
        self.next_descriptor = next;

        ret
    }
}

struct TxDevice {
    descriptors: Box<[Volatile<tx::TxDescriptor>]>,
    next_descriptor: usize,
}

impl TxDevice {
    fn new(config: TxConfig) -> TxDevice {
        use self::tx::TxDescriptor;

        let descriptor_num = config.number_of_descriptors;
        let mut descriptors = Vec::with_capacity(descriptor_num);

        for i in 0..descriptor_num {
            let mut descriptor = TxDescriptor::empty();
            if i == descriptor_num - 1 {
                descriptor.set_end_of_ring(true);
            }
            descriptors.push(Volatile::new(descriptor));
        }

        TxDevice {
            descriptors: descriptors.into_boxed_slice(),
            next_descriptor: 0,
        }
    }

    fn descriptor_available(&self) -> bool {
        !self.descriptors[self.next_descriptor].read().own()
    }

    pub fn insert(&mut self, data: Box<[u8]>) {
        while self.descriptors[self.next_descriptor].read().own() {}
        self.descriptors[self.next_descriptor].update(|d| d.set_data(data));
        self.next_descriptor = (self.next_descriptor + 1) % self.descriptors.len();

        self.cleanup();
    }

    pub fn front_of_queue(&self) -> &Volatile<tx::TxDescriptor> {
        self.descriptors.first().unwrap()
    }

    pub fn cleanup(&mut self) {
        let mut c = 0;
        for descriptor in self.descriptors.iter_mut() {
            descriptor.update(|d| {
                if !d.own() && d.buffer().is_some() {
                    c += 1;
                }
            });
        }
        if c > 0 {
            // println!("cleaned up {} packets", c);
        }
    }
}

/// Configures the package reception buffer.
pub struct RxConfig {
    buffer_size: usize,
    number_of_descriptors: usize,
    default_descriptor_buffer_size: usize,
}

impl RxConfig {
    fn descriptor_buffer_size(&self, descriptor_index: usize) -> usize {
        let number_of_default_descriptors = self.number_of_descriptors - 1;
        if descriptor_index == number_of_default_descriptors {
            self.buffer_size - number_of_default_descriptors * self.default_descriptor_buffer_size
        } else {
            self.default_descriptor_buffer_size
        }
    }

    fn descriptor_buffer_offset(&self, descriptor_index: usize) -> usize {
        descriptor_index * self.default_descriptor_buffer_size
    }
}

impl Default for RxConfig {
    fn default() -> RxConfig {
        let number_of_descriptors = 128;
        let default_descriptor_buffer_size = 64;
        RxConfig {
            buffer_size: default_descriptor_buffer_size * (number_of_descriptors - 1) + MTU,
            number_of_descriptors: number_of_descriptors,
            default_descriptor_buffer_size: default_descriptor_buffer_size,
        }
    }
}

/// Configures the package transmission buffer.
pub struct TxConfig {
    number_of_descriptors: usize,
}

impl Default for TxConfig {
    fn default() -> TxConfig {
        TxConfig {
            number_of_descriptors: 64,
        }
    }
}
