use alloc::boxed::Box;
use collections::vec::Vec;

use board::{rcc, syscfg};
use board::ethernet_dma::{self, EthernetDma};
use board::ethernet_mac::EthernetMac;
use embedded::interfaces::gpio;
use smoltcp;
use volatile::Volatile;

mod init;
mod phy;
mod rx;
mod tx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Exhausted,
    Checksum,
    Truncated,
    Parsing(smoltcp::Error),
}

impl From<smoltcp::Error> for Error {
    fn from(err: smoltcp::Error) -> Error {
        Error::Parsing(err)
    }
}

const MTU: usize = 1536;

pub struct EthernetDevice {
    rx: RxDevice,
    ethernet_dma: &'static mut EthernetDma,
}

impl EthernetDevice {
    pub fn new(rx_config: RxConfig,
               rcc: &mut rcc::Rcc,
               syscfg: &mut syscfg::Syscfg,
               gpio: &mut gpio::Gpio,
               ethernet_mac: &'static mut EthernetMac,
               ethernet_dma: &'static mut EthernetDma)
               -> Result<EthernetDevice, init::Error> {
        init::init(rcc, syscfg, gpio, ethernet_mac, ethernet_dma)?;

        let rx_device = RxDevice::new(rx_config)?;

        let mut srl = ethernet_dma::Dmardlar::default();
        srl.set_srl(&rx_device.descriptors[0] as *const Volatile<_> as u32);
        ethernet_dma.dmardlar.write(srl);

        init::start(ethernet_mac, ethernet_dma);

        Ok(EthernetDevice {
            rx: rx_device,
            ethernet_dma: ethernet_dma,
        })
    }

    pub fn dump_next_packet(&mut self) -> Result<(), Error> {
        use smoltcp::wire::{EthernetFrame, EthernetProtocol};

        let missed_packets = self.ethernet_dma.dmamfbocr.read().mfc();
        if missed_packets > 20 {
            println!("missed packets: {}", missed_packets);
        }

        let &mut EthernetDevice { ref mut rx, .. } = self;

        rx.receive(|data| -> Result<_, Error> {
                let eth_frame = EthernetFrame::new(data)?;
                match eth_frame.ethertype() {
                    EthernetProtocol::Arp => {
                        use smoltcp::wire::{ArpPacket, ArpRepr};
                        let arp_packet = ArpPacket::new(eth_frame.payload())?;
                        let arp_repr = ArpRepr::parse(&arp_packet)?;
                        println!("Arp: {:?}", arp_repr);
                    }
                    EthernetProtocol::Ipv4 => {
                        use smoltcp::wire::{Ipv4Packet, Ipv4Repr};
                        let ipv4_packet = Ipv4Packet::new(eth_frame.payload())?;
                        let ipv4_repr = Ipv4Repr::parse(&ipv4_packet)?;

                        println!("Ipv4: {:?}", ipv4_repr);
                    }
                    _ => println!("{:?}", eth_frame.ethertype()),
                }
                Ok(())
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
            Err(Error::Exhausted);
        } else {
            let mut last_descriptor = descriptor;
            let mut i = 0;
            while !last_descriptor.is_last_descriptor() {
                i += 1;
                last_descriptor = self.descriptors[descriptor_index + i].read();
            }
            if last_descriptor.error() {
                Err(smoltcp::Error::Truncated)
            } else {
                let offset = self.config.descriptor_buffer_offset(descriptor_index);
                let len = last_descriptor.frame_len();
                print!("len {}: ", len);
                Ok(&self.buffer[offset..(offset + len)])
            }
        }
    }

    fn receive<T, F>(&mut self, f: F) -> Result<T, Error>
        where F: FnOnce(&[u8]) -> T
    {
        let ret = {
            let data = self.packet_data(self.next_descriptor)?;
            f(data)
        };
        loop {
            let next = (self.next_descriptor + 1) % self.descriptors.len();
            let descriptor = self.descriptors[self.next_descriptor].read();
            self.descriptors[self.next_descriptor].update(|d| d.reset());
            self.next_descriptor = next;
            if descriptor.is_last_descriptor() {
                break;
            }
        }
        Ok(ret)
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
        let number_of_descriptors = 30;
        let default_descriptor_buffer_size = 0x100;
        RxConfig {
            buffer_size: default_descriptor_buffer_size * (number_of_descriptors - 1) + MTU,
            number_of_descriptors: number_of_descriptors,
            default_descriptor_buffer_size: default_descriptor_buffer_size,
        }
    }
}
