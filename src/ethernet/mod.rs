use core::fmt;
use alloc::boxed::Box;
use alloc::Vec;

use board::{rcc, syscfg};
use board::ethernet_dma::{self, EthernetDma};
use board::ethernet_mac::{self, EthernetMac};
use embedded::interfaces::gpio;
use volatile::Volatile;

use smoltcp::wire::{EthernetAddress, Ipv4Address, IpCidr, Ipv4Cidr};
use smoltcp::phy::{Device, DeviceCapabilities};
use smoltcp::iface::{EthernetInterface, EthernetInterfaceBuilder};
use smoltcp::time::Instant;

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

pub struct EthernetDevice {
    rx: RxDevice,
    tx: TxDevice,
    ethernet_dma: &'static mut EthernetDma,
    ethernet_address: EthernetAddress,
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
        ethernet_address: EthernetAddress,
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

        let eth_bytes = ethernet_address.as_bytes();
        let mut mac0_low = ethernet_mac::Maca0lr::default();
        mac0_low.set_maca0l(LittleEndian::read_u32(&eth_bytes[..4]));
        ethernet_mac.maca0lr.write(mac0_low);
        let mut mac0_high = ethernet_mac::Maca0hr::default();
        mac0_high.set_maca0h(LittleEndian::read_u16(&eth_bytes[4..]));
        ethernet_mac.maca0hr.write(mac0_high);

        init::start(ethernet_mac, ethernet_dma);
        Ok(EthernetDevice {
            rx: rx_device,
            tx: tx_device,
            ethernet_dma: ethernet_dma,
            ethernet_address: ethernet_address,
        })
    }

    pub fn into_interface<'a>(self, ip_address: Ipv4Address) -> EthernetInterface<'a, 'a, Self> {
        use smoltcp::iface::NeighborCache;
        use alloc::BTreeMap;

        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        let ethernet_address = self.ethernet_address;
        let interface_builder = EthernetInterfaceBuilder::new(self);
        let interface_builder = interface_builder.ethernet_addr(ethernet_address);
        let ip_cidr = IpCidr::Ipv4(Ipv4Cidr::new(ip_address, 0));
        let interface_builder = interface_builder.ip_addrs(vec![ip_cidr]);
        let interface_builder = interface_builder.neighbor_cache(neighbor_cache);
        interface_builder.finalize()
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
        if !self.rx.new_data_received() { return None }
        let rx = RxToken { rx: &mut self.rx, };
        let tx = TxToken { tx: &mut self.tx, ethernet_dma: &mut self.ethernet_dma, };
        Some((rx, tx))
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        if !self.tx.descriptor_available() { return None }
        Some(TxToken { tx: &mut self.tx, ethernet_dma: &mut self.ethernet_dma, })
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
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> ::smoltcp::Result<R>
        where F: FnOnce(&[u8]) -> ::smoltcp::Result<R>
    {
        self.rx.receive(f)
    }
}

pub struct TxToken<'a> {
    tx: &'a mut TxDevice,
    ethernet_dma: &'a mut EthernetDma,
}

impl<'a> ::smoltcp::phy::TxToken for TxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, len: usize, f: F) -> ::smoltcp::Result<R>
        where F: FnOnce(&mut [u8]) -> ::smoltcp::Result<R>
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
        match self.ethernet_dma.dmasr.read().tps() {
            // stopped
            0b000 => panic!("stopped"),
            // running
            0b001 | 0b010 | 0b011 | 0b111 => {}
            // suspended
            0b110 => {
                // write poll demand register
                let mut poll_demand = ethernet_dma::Dmatpdr::default();
                poll_demand.set_tpd(0); // any value
                self.ethernet_dma.dmatpdr.write(poll_demand);
            }
            _ => unreachable!(),
        }
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

    fn new_data_received(&self) -> bool {
        let descriptor = self.descriptors[self.next_descriptor].read();
        !descriptor.own() && descriptor.is_first_descriptor()
    }

    fn receive<T, F>(&mut self, f: F) -> ::smoltcp::Result<T>
    where
        F: FnOnce(&[u8]) -> ::smoltcp::Result<T>,
    {
        let descriptor_index = self.next_descriptor;
        let descriptor = self.descriptors[descriptor_index].read();

        if descriptor.own() || !descriptor.is_first_descriptor() {
            return Err(::smoltcp::Error::Exhausted);
        }
        if let rx::ChecksumResult::Error(_, _) = descriptor.checksum_result() {
            return Err(::smoltcp::Error::Checksum);
        }

        // find the last descriptor belonging to the received packet
        let mut last_descriptor = descriptor;
        let mut i = 0;
        while !last_descriptor.is_last_descriptor() {
            i += 1;
            // Descriptors wrap around, but we don't want packets to wrap around. So we require
            // that the last descriptor in the list is large enough to hold all received packets.
            // This assertion checks that no wraparound occurs.
            assert!(descriptor_index + i < self.descriptors.len(), "buffer of last descriptor in \
                list must be large enough to hold received packets without wrap-around");
            last_descriptor = self.descriptors[descriptor_index + i].read();
            if last_descriptor.own() {
                return Err(::smoltcp::Error::Exhausted); // packet is not fully received
            }
        }

        // check for errors
        let mut error = None;
        if last_descriptor.error() {
            if last_descriptor.crc_error() {
                println!("crc_error");
            }
            if last_descriptor.receive_error() {
                println!("receive_error");
            }
            if last_descriptor.watchdog_timeout_error() {
                println!("watchdog_timeout_error");
            }
            if last_descriptor.late_collision_error() {
                println!("late_collision_error");
            }
            if last_descriptor.giant_frame_error() {
                println!("giant_frame_error");
            }
            if last_descriptor.overflow_error() {
                println!("overflow_error");
            }
            if last_descriptor.descriptor_error() {
                println!("descriptor_error");
            }
            error = Some(::smoltcp::Error::Truncated);
        }

        let ret = match error {
            Some(error) => Err(error),
            None => {
                // read data and pass it to processing function
                let offset = self.config.descriptor_buffer_offset(descriptor_index);
                let len = last_descriptor.frame_len();
                let data = &self.buffer[offset..(offset + len)];
                f(data)
            }
        };

        // reset descriptor(s) and update next_descriptor
        let mut next = descriptor_index;
        loop {
            let descriptor = self.descriptors[next].read();
            self.descriptors[next].update(|d| d.reset());
            next = (next + 1) % self.descriptors.len();
            if descriptor.is_last_descriptor() { break }
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
