use alloc::boxed::Box;
use collections::{Vec, BTreeMap};
use collections::borrow::Cow;

use board::{rcc, syscfg};
use board::ethernet_dma::{self, EthernetDma};
use board::ethernet_mac::{self, EthernetMac};
use embedded::interfaces::gpio;
use volatile::Volatile;
use net::{self, HeapTxPacket};
use net::ipv4::{Ipv4Address, Ipv4Header};
use net::udp::UdpHeader;
use net::ethernet::{EthernetAddress, EthernetPacket};

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
    Parsing(net::ParseError),
    Initialization(init::Error),
}

impl From<net::ParseError> for Error {
    fn from(err: net::ParseError) -> Error {
        Error::Parsing(err)
    }
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

const MTU: usize = 1536;
const ETH_ADDR: EthernetAddress = EthernetAddress::new([0x00, 0x08, 0xdc, 0xab, 0xcd, 0xef]);

pub enum Packet<'a> {
    Udp(Udp<'a>),
}

impl<'a> Packet<'a> {
    pub fn udp_port(&self, port: u16) -> Option<&Udp> {
        match self {
            &Packet::Udp(ref udp) => {
                if udp.udp_header.dst_port == port {
                    Some(udp)
                } else {
                    None
                }
            }
        }
    }

    pub fn bind_udp<F>(&self, port: u16, f: F)
        where F: FnOnce(&Udp)
    {
        if let Some(udp) = self.udp_port(port) {
            f(udp)
        }
    }
}

pub struct Udp<'a> {
    pub ip_header: Ipv4Header,
    pub udp_header: UdpHeader,
    pub payload: &'a [u8],
}

pub struct EthernetDevice {
    rx: RxDevice,
    tx: TxDevice,
    ethernet_dma: &'static mut EthernetDma,
    ipv4_addr: Option<Ipv4Address>,
    requested_ipv4_addr: Option<Ipv4Address>,
    last_discover_at: usize,
    arp_cache: BTreeMap<Ipv4Address, EthernetAddress>,
}

impl EthernetDevice {
    pub fn new(rx_config: RxConfig,
               tx_config: TxConfig,
               rcc: &mut rcc::Rcc,
               syscfg: &mut syscfg::Syscfg,
               gpio: &mut gpio::Gpio,
               ethernet_mac: &'static mut EthernetMac,
               ethernet_dma: &'static mut EthernetDma)
               -> Result<EthernetDevice, Error> {
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
            arp_cache: BTreeMap::new(),
        };

        device.send_dhcp_discover()?;

        Ok(device)
    }

    fn start_send(&mut self) {
        match self.ethernet_dma.dmasr.read().tps() { // transmit process state
            0b000 => panic!("stopped"), // stopped
            0b001 | 0b010 | 0b011 | 0b111 => {
                println!("running");
            } // running
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

    pub fn send_dhcp_discover(&mut self) -> Result<(), Error> {
        use net::{dhcp, WriteOut};
        use system_clock;

        if system_clock::ticks() - self.last_discover_at < 5000 {
            return Ok(());
        }

        let packet = dhcp::new_discover_msg(ETH_ADDR);
        let mut tx_packet = HeapTxPacket::new(packet.len());
        packet.write_out(&mut tx_packet)?;

        self.tx.insert(tx_packet.into_boxed_slice());
        self.start_send();

        self.last_discover_at = system_clock::ticks();

        Ok(())
    }

    pub fn with_next_packet<F>(&mut self, f: F) -> Result<(), Error>
        where F: FnOnce(Packet) -> Option<Cow<[u8]>>
    {
        let missed_packets = self.ethernet_dma.dmamfbocr.read().mfc();
        if missed_packets > 20 {
            println!("missed packets: {}", missed_packets);
        }

        if self.ipv4_addr.is_none() && self.requested_ipv4_addr.is_none() {
            self.send_dhcp_discover()?;
        }

        let reply = self.process_next_packet(f)?;
        if let Some(tx_packet) = reply {
            self.tx.insert(tx_packet);
            self.start_send();
        }

        Ok(())
    }

    fn process_next_packet<F>(&mut self, f: F) -> Result<Option<Box<[u8]>>, Error>
        where F: FnOnce(Packet) -> Option<Cow<[u8]>>
    {
        let &mut EthernetDevice {
                     ref mut rx,
                     ref mut ipv4_addr,
                     ref mut requested_ipv4_addr,
                     ref mut arp_cache,
                     ..
                 } = self;

        rx.receive(|data| -> Result<_, Error> {
                use net;
                use net::ethernet::EthernetKind;
                use net::arp;
                use net::ipv4::{Ipv4Packet, Ipv4Kind};
                use net::udp::{UdpPacket, UdpKind};
                use net::dhcp::{self, DhcpPacket, DhcpType};
                use net::icmp::IcmpType;

                let EthernetPacket { header: _, payload } = net::parse(data)?;

                match payload {
                    // DHCP offer or ack for us
                    EthernetKind::Ipv4(Ipv4Packet {
                                           header: _,
                                           payload: Ipv4Kind::Udp(UdpPacket {
                                                             header: _,
                                                             payload: UdpKind::Dhcp(DhcpPacket {
                                                                               mac,
                                                                               operation,
                                                                               ..
                                                                           }),
                                                         }),
                                       }) if mac == ETH_ADDR => {
                        match operation {
                            DhcpType::Offer { ip, dhcp_server_ip } => {
                                println!("DHCP offer: {:?}", ip);
                                if requested_ipv4_addr.is_none() {
                                    *requested_ipv4_addr = Some(ip);
                                    let reply = dhcp::new_request_msg(ETH_ADDR, ip, dhcp_server_ip);
                                    return Ok(Some(HeapTxPacket::write_out(reply)?
                                                       .into_boxed_slice()));
                                }
                            }
                            DhcpType::Ack { ip } => {
                                assert_eq!(Some(ip), *requested_ipv4_addr);
                                println!("DHCP ack: {:?}", ip);
                                *ipv4_addr = Some(ip);
                            }
                            op => panic!("Unknown dhcp operation {:?}", op),
                        }
                    }

                    // Arp for our ip
                    EthernetKind::Arp(arp) if Some(arp.dst_ip) == *ipv4_addr => {
                        use net::arp::ArpOperation;

                        arp_cache.insert(arp.src_ip, arp.src_mac);

                        match arp.operation {
                            ArpOperation::Request => {
                                println!("arp request for our ip from {:?} ({:?})",
                                         arp.src_ip,
                                         arp.src_mac);
                                let reply = arp.response_packet(ETH_ADDR);
                                return Ok(Some(HeapTxPacket::write_out(reply)?.into_boxed_slice()));
                            }
                            ArpOperation::Response => {
                                println!("arp response from {:?} for ip {:?}",
                                         arp.src_mac,
                                         arp.src_ip);
                            }
                        }
                    }

                    // ICMP echo request
                    EthernetKind::Ipv4(Ipv4Packet {
                                           header: ip_header,
                                           payload: Ipv4Kind::Icmp(icmp),
                                       }) if Some(ip_header.dst_addr) == *ipv4_addr => {
                        match icmp.type_ {
                            IcmpType::EchoRequest { .. } => {
                                //println!("icmp echo request");
                                let src_ip = ip_header.dst_addr;
                                let dst_ip = ip_header.src_addr;
                                if let Some(&dst_mac) = arp_cache.get(&dst_ip) {
                                    let reply =
                                        icmp.echo_reply_packet(ETH_ADDR, dst_mac, src_ip, dst_ip);
                                    return Ok(Some(HeapTxPacket::write_out(reply)?
                                                       .into_boxed_slice()));
                                } else {
                                    let arp_request =
                                        arp::new_request_packet(ETH_ADDR, src_ip, dst_ip);
                                    return Ok(Some(HeapTxPacket::write_out(arp_request)?
                                                       .into_boxed_slice()));
                                }
                            }
                            IcmpType::EchoReply {
                                id,
                                sequence_number,
                            } => {
                                println!("icmp echo reply {{id: {}, sequence_number: {}}}",
                                         id,
                                         sequence_number);
                            }
                        }
                    }

                    // other Udp packet
                    EthernetKind::Ipv4(Ipv4Packet {
                                           header: ip_header,
                                           payload: Ipv4Kind::Udp(UdpPacket {
                                                             header: udp_header,
                                                             payload: UdpKind::Unknown(payload),
                                                         }),
                                       }) if Some(ip_header.dst_addr) == *ipv4_addr => {
                        if let Some(reply_payload) =
                            f(Packet::Udp(Udp {
                                              ip_header,
                                              udp_header,
                                              payload,
                                          })) {
                            let src_ip = ip_header.dst_addr;
                            let dst_ip = ip_header.src_addr;
                            if let Some(&dst_mac) = arp_cache.get(&dst_ip) {
                                let packet = net::udp::new_udp_packet(ETH_ADDR,
                                                                      dst_mac,
                                                                      src_ip,
                                                                      dst_ip,
                                                                      udp_header.dst_port,
                                                                      udp_header.src_port,
                                                                      reply_payload);
                                return Ok(Some(HeapTxPacket::write_out(packet)?
                                                   .into_boxed_slice()));
                            } else {
                                let arp_request = arp::new_request_packet(ETH_ADDR, src_ip, dst_ip);
                                return Ok(Some(HeapTxPacket::write_out(arp_request)?
                                                   .into_boxed_slice()));
                            }
                        };
                    }

                    _ => {} //{println!("{:?}", other)},
                }

                Ok(None)
            })?
    }
}

impl Drop for EthernetDevice {
    fn drop(&mut self) {
        // TODO stop ethernet device and wait for idle
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

    fn packet_data(&self, descriptor_index: usize) -> Result<&[u8], Error> {
        let descriptor = self.descriptors[descriptor_index].read();
        if descriptor.own() {
            return Err(Error::Exhausted);
        }
        if let rx::ChecksumResult::Error(header, payload) = descriptor.checksum_result() {
            println!("checksum error {} {}", header, payload);
            return Err(Error::Checksum);
        }

        let mut last_descriptor = descriptor;
        let mut i = 0;
        while !last_descriptor.is_last_descriptor() {
            i += 1;
            assert!(descriptor_index + i < self.descriptors.len(),
                    "last descriptor buffer too small"); // no wrap around
            last_descriptor = self.descriptors[descriptor_index + i].read();
            if last_descriptor.own() {
                return Err(Error::Exhausted); // packet is not fully received
            }
        }
        if last_descriptor.error() {
            Err(Error::Truncated)
        } else {
            assert!(descriptor.is_first_descriptor());
            let offset = self.config.descriptor_buffer_offset(descriptor_index);
            let len = last_descriptor.frame_len();
            Ok(&self.buffer[offset..(offset + len)])
        }
    }

    fn receive<T, F>(&mut self, f: F) -> Result<T, Error>
        where F: FnOnce(&[u8]) -> T
    {
        let descriptor_index = self.next_descriptor;
        let ret = self.packet_data(descriptor_index).map(f);

        if let Err(Error::Exhausted) = ret {
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

        // println!("insert tx packet");
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
        TxConfig { number_of_descriptors: 64 }
    }
}
