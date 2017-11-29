use core::fmt;
use alloc::boxed::Box;
use alloc::Vec;

use board::{rcc, syscfg};
use board::ethernet_dma::{self, EthernetDma};
use board::ethernet_mac::{self, EthernetMac};
use embedded::interfaces::gpio;
use volatile::Volatile;

use smoltcp::wire::{EthernetAddress, Ipv4Address};
use smoltcp::phy::{Device, DeviceCapabilities};
use smoltcp::iface::EthernetInterface;

mod init;
mod phy;
mod rx;
mod tx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Exhausted,
    Checksum,
    Truncated,
    NoIp,
    Unknown,
    Initialization(init::Error),
}

impl From<init::Error> for Error {
    fn from(err: init::Error) -> Error {
        Error::Initialization(err)
    }
}

impl From<()> for Error {
    fn from(_: ()) -> Error {
        Error::Unknown
    }
}

pub const MTU: usize = 1536;
const ETH_ADDR: EthernetAddress = EthernetAddress([0x00, 0x08, 0xdc, 0xab, 0xcd, 0xef]);

pub struct EthernetDevice {
    rx: RxDevice,
    tx: TxDevice,
    ethernet_dma: &'static mut EthernetDma,
    ipv4_addr: Option<Ipv4Address>,
    requested_ipv4_addr: Option<Ipv4Address>,
    last_discover_at: usize,
}

impl EthernetDevice {
    pub fn new(
        rx_config: RxConfig,
        tx_config: TxConfig,
        rcc: &mut rcc::Rcc,
        syscfg: &mut syscfg::Syscfg,
        gpio: &mut gpio::Gpio,
        ethernet_mac: &'static mut EthernetMac,
        ethernet_dma: &'static mut EthernetDma,
    ) -> Result<EthernetDevice, Error> {
        use byteorder::{ByteOrder, LittleEndian};

        init::init(rcc, syscfg, gpio, ethernet_mac, ethernet_dma)?;

        let rx_device = RxDevice::new(rx_config)?;
        let tx_device = TxDevice::new(tx_config);

        let mut srl = ethernet_dma::Dmardlar::default();
        srl.set_srl(&rx_device.descriptors[0] as *const Volatile<_> as u32);
        ethernet_dma.dmardlar.write(srl);

        let mut stl = ethernet_dma::Dmatdlar::default();
        stl.set_stl(tx_device.front_of_queue() as *const Volatile<_> as u32);
        ethernet_dma.dmatdlar.write(stl);

        let eth_bytes = ETH_ADDR.as_bytes();
        let mut mac0_low = ethernet_mac::Maca0lr::default();
        mac0_low.set_maca0l(LittleEndian::read_u32(&eth_bytes[..4]));
        ethernet_mac.maca0lr.write(mac0_low);
        let mut mac0_high = ethernet_mac::Maca0hr::default();
        mac0_high.set_maca0h(LittleEndian::read_u16(&eth_bytes[4..]));
        ethernet_mac.maca0hr.write(mac0_high);

        init::start(ethernet_mac, ethernet_dma);
        let mut device = EthernetDevice {
            rx: rx_device,
            tx: tx_device,
            ethernet_dma: ethernet_dma,
            ipv4_addr: None,
            requested_ipv4_addr: None,
            last_discover_at: 0,
        };
        Ok(device)
    }

    pub fn into_interface<'a>(self) -> EthernetInterface<'a, 'a, Self> {
        use smoltcp::iface::NeighborCache;
        use alloc::BTreeMap;

        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        EthernetInterface::new(self, neighbor_cache, ETH_ADDR, [], None)
    }

    fn start_send(&mut self) {
        match self.ethernet_dma.dmasr.read().tps() {
            // transmit process state
            0b000 => panic!("stopped"), // stopped
            0b001 | 0b010 | 0b011 | 0b111 => {
                println!("running");
            }
            // running
            0b110 => {
                // suspended
                if !self.tx.queue_empty() {
                    // write poll demand register
                    let mut poll_demand = ethernet_dma::Dmatpdr::default();
                    poll_demand.set_tpd(0); // any value
                    self.ethernet_dma.dmatpdr.write(poll_demand);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Drop for EthernetDevice {
    fn drop(&mut self) {
        // TODO stop ethernet device and wait for idle
    }
}

impl<'a> Device<'a> for EthernetDevice {
    type RxToken = RxToken<'a>;
    type TxToken = TxToken<'a>;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let rx = RxToken { rx: &mut self.rx, };
        let tx = TxToken { tx: &mut self.tx, };
        Some((rx, tx))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken { tx: &mut self.tx, })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut capabilities = DeviceCapabilities::default();
        capabilities.max_transmission_unit = MTU;
        capabilities
    }
}

pub struct RxToken<'a> {
    rx: &'a mut RxDevice,
}

impl<'a> ::smoltcp::phy::RxToken for RxToken<'a> {
    fn consume<R, F>(self, _timestamp: u64, f: F) -> ::smoltcp::Result<R>
        where F: FnOnce(&[u8]) -> ::smoltcp::Result<R>
    {
        self.rx.receive(f)
    }
}

pub struct TxToken<'a> {
    tx: &'a mut TxDevice,
}

impl<'a> ::smoltcp::phy::TxToken for TxToken<'a> {
    fn consume<R, F>(self, _timestamp: u64, len: usize, f: F) -> ::smoltcp::Result<R>
        where F: FnOnce(&mut [u8]) -> ::smoltcp::Result<R>
    {
        let mut data = vec![0; len].into_boxed_slice();
        let ret = f(&mut data)?;
        self.tx.insert(data);
        Ok(ret)
    }
}

pub struct PortInUse<F> {
    pub tcp: bool,
    pub port: u16,
    pub f: F,
}

impl<F> PortInUse<F> {
    pub fn new(tcp: bool, port: u16, f: F) -> PortInUse<F> {
        PortInUse { tcp, port, f }
    }
}

impl<F> fmt::Debug for PortInUse<F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{} port {} already in use",
            if self.tcp { "TCP" } else { "UDP" },
            self.port
        )
    }
}

struct RxDevice {
    config: RxConfig,
    buffer: Box<[u8]>,
    descriptors: Box<[Volatile<rx::RxDescriptor>]>,
    next_descriptor: usize,
}

impl RxDevice {
    fn new(config: RxConfig) -> Result<RxDevice, init::Error> {
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

    fn packet_data(&self, descriptor_index: usize) -> ::smoltcp::Result<&[u8]> {
        let descriptor = self.descriptors[descriptor_index].read();
        if descriptor.own() {
            return Err(::smoltcp::Error::Exhausted);
        }
        if let rx::ChecksumResult::Error(header, payload) = descriptor.checksum_result() {
            println!("checksum error {} {}", header, payload);
            return Err(::smoltcp::Error::Checksum);
        }

        let mut last_descriptor = descriptor;
        let mut i = 0;
        while !last_descriptor.is_last_descriptor() {
            i += 1;
            assert!(
                descriptor_index + i < self.descriptors.len(),
                "last descriptor buffer too small"
            ); // no wrap around
            last_descriptor = self.descriptors[descriptor_index + i].read();
            if last_descriptor.own() {
                return Err(::smoltcp::Error::Exhausted); // packet is not fully received
            }
        }
        if last_descriptor.error() {
            Err(::smoltcp::Error::Truncated)
        } else {
            assert!(descriptor.is_first_descriptor());
            let offset = self.config.descriptor_buffer_offset(descriptor_index);
            let len = last_descriptor.frame_len();
            Ok(&self.buffer[offset..(offset + len)])
        }
    }

    fn receive<T, F>(&mut self, f: F) -> ::smoltcp::Result<T>
    where
        F: FnOnce(&[u8]) -> ::smoltcp::Result<T>,
    {
        let descriptor_index = self.next_descriptor;
        let ret = self.packet_data(descriptor_index).and_then(f);

        if let Err(::smoltcp::Error::Exhausted) = ret {
            return ret;
        }

        // reset descriptor(s) and update next_descriptor
        let mut next = (descriptor_index + 1) % self.descriptors.len();
        if ret.is_ok() {
            // handle subsequent descriptors if descriptor is not last_descriptor
            let mut descriptor = self.descriptors[descriptor_index].read();
            while !descriptor.is_last_descriptor() {
                descriptor = self.descriptors[next].read();
                self.descriptors[next].update(|d| d.reset());
                next = (next + 1) % self.descriptors.len();
            }
        }
        self.descriptors[descriptor_index].update(|d| d.reset());
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

    pub fn insert(&mut self, data: Box<[u8]>) {
        while self.descriptors[self.next_descriptor].read().own() {}
        self.descriptors[self.next_descriptor].update(|d| d.set_data(data));
        self.next_descriptor = (self.next_descriptor + 1) % self.descriptors.len();

        self.cleanup();
    }

    pub fn front_of_queue(&self) -> &Volatile<tx::TxDescriptor> {
        self.descriptors.first().unwrap()
    }

    pub fn queue_empty(&self) -> bool {
        self.descriptors.iter().all(|d| !d.read().own())
    }

    pub fn cleanup(&mut self) {
        let mut c = 0;
        for descriptor in self.descriptors.iter_mut() {
            descriptor.update(|d| if !d.own() && d.buffer().is_some() {
                c += 1;
            });
        }
        if c > 0 {
            // println!("cleaned up {} packets", c);
        }
    }
}

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
