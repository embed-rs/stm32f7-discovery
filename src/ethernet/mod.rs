use alloc::boxed::Box;
use collections::vec::Vec;

use board::{rcc, syscfg};
use board::ethernet_dma::{self, EthernetDma};
use board::ethernet_mac::EthernetMac;
use embedded::interfaces::gpio;
use volatile::Volatile;

mod init;
mod phy;
mod rx;
mod tx;

pub struct EthernetDevice {
    rx_config: RxConfig,
    rx_buffer: Box<[u8]>,
    rx_descriptors: Box<[Volatile<rx::RxDescriptor>]>,
}

impl EthernetDevice {
    pub fn new(rx_config: RxConfig,
               rcc: &mut rcc::Rcc,
               syscfg: &mut syscfg::Syscfg,
               gpio: &mut gpio::Gpio,
               ethernet_mac: &'static mut EthernetMac,
               ethernet_dma: &'static mut EthernetDma)
               -> Result<EthernetDevice, init::Error> {
        use self::rx::RxDescriptor;

        init::init(rcc, syscfg, gpio, ethernet_mac, ethernet_dma)?;

        let rx_buffer = vec![0; rx_config.buffer_size].into_boxed_slice();
        let descriptor_num = rx_config.number_of_descriptors;
        let mut rx_descriptors = Vec::with_capacity(descriptor_num);

        for i in 0..descriptor_num {
            let buffer_offset = rx_config.descriptor_buffer_offset(i);
            let buffer_start = &rx_buffer[buffer_offset];
            let buffer_size = rx_config.descriptor_buffer_size(i);

            let descriptor = RxDescriptor::new(buffer_start, buffer_size);
            rx_descriptors.push(Volatile::new(descriptor));
        }

        // convert Vec to boxed slice to ensure that no reallocations occur; this allows us
        // to safely link the descriptors.
        let mut rx_descriptors = rx_descriptors.into_boxed_slice();
        link_descriptors(&mut rx_descriptors);

        fn link_descriptors(descriptors: &mut [Volatile<RxDescriptor>]) {
            let mut iter = descriptors.iter_mut().peekable();
            while let Some(descriptor) = iter.next() {
                if let Some(next) = iter.peek() {
                    descriptor.update(|d| d.set_next(*next));
                }
            }
        }

        let eth_device = EthernetDevice {
            rx_config: rx_config,
            rx_buffer: rx_buffer,
            rx_descriptors: rx_descriptors,
        };

        let mut srl = ethernet_dma::Dmardlar::default();
        srl.set_srl(&eth_device.rx_descriptors[0] as *const Volatile<_> as u32);
        ethernet_dma.dmardlar.write(srl);

        init::start(ethernet_mac, ethernet_dma);
        Ok(eth_device)
    }

    fn rx_packet_data(&self, descriptor_index: usize) -> Result<&[u8], ::smoltcp::Error> {
        use core::convert::TryFrom;
        let descriptor = self.rx_descriptors[descriptor_index].read();
        if descriptor.own() {
            Err(::smoltcp::Error::Exhausted)
        } else {
            let offset = self.rx_config.descriptor_buffer_offset(descriptor_index);
            // TODO multiple descriptors
            let len = usize::try_from(descriptor.buffer_1_size()).unwrap();
            Ok(&self.rx_buffer[offset..(offset + len)])
        }
    }

    pub fn dump_packet_data(&self) -> Result<(), ::smoltcp::Error> {
        use smoltcp::wire::{EthernetFrame, EthernetProtocol};

        for i in 0..self.rx_descriptors.len() {
            let eth_frame = EthernetFrame::new(self.rx_packet_data(i)?)?;
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

                    println!("Ipv4: {:#?}", ipv4_repr);
                }
                _ => println!("{:?}", eth_frame.ethertype()),
            }
        }

        Ok(())
    }
}

impl Drop for EthernetDevice {
    fn drop(&mut self) {
        // TODO stop ethernet device and wait for idle
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
        let mtu = 1500;
        let number_of_descriptors = 10;
        let default_descriptor_buffer_size = 0x100;
        RxConfig {
            buffer_size: default_descriptor_buffer_size * (number_of_descriptors - 1) + mtu,
            number_of_descriptors: number_of_descriptors,
            default_descriptor_buffer_size: default_descriptor_buffer_size,
        }
    }
}
