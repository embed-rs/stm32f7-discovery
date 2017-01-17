use board::ethernet_mac::{self, EthernetMac};
use bit_field::BitField;
use system_clock;

const LAN8742A_PHY_ADDRESS: u8 = 0;

const BASIC_CONTROL_REG: u8 = 0;
const BASIC_STATUS_REG: u8 = 1; // basic status register
const SPECIAL_STATUS_REG: u8 = 31; // special status register

const PHY_RESET: u16 = 1 << 15;
const AUTONEGOTIATION_ENABLE: u16 = 1 << 12;
const AUTONEGOTIATION_RESTART: u16 = 1 << 9;

const TIMEOUT: usize = 5_000;

#[derive(Debug)]
pub enum Error {
    LinkTimeout,
    AutoNegotiationTimeout,
}

pub struct AutoNegotiationResult {
    pub duplex: bool,
    pub speed: Speed,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Speed {
    Speed10M,
    Speed100M,
}

pub fn init(ethernet_mac: &mut EthernetMac) -> Result<AutoNegotiationResult, Error> {
    // reset PHY
    phy_write(ethernet_mac,
              LAN8742A_PHY_ADDRESS,
              BASIC_CONTROL_REG,
              PHY_RESET);
    // wait 0.5s
    system_clock::wait(500);
    // wait for reset bit auto clear
    while phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_CONTROL_REG) & PHY_RESET != 0 {}

    // wait for link bit
    let ticks = system_clock::ticks();
    while !phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(2) {
        if system_clock::ticks() - ticks > TIMEOUT {
            return Err(Error::LinkTimeout); // timeout
        }
    }

    // enable auto-negotiation
    phy_write(ethernet_mac,
              LAN8742A_PHY_ADDRESS,
              BASIC_CONTROL_REG,
              AUTONEGOTIATION_ENABLE | AUTONEGOTIATION_RESTART);

    // wait until auto-negotiation complete bit is set
    let ticks = system_clock::ticks();
    while !phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(5) {
        if system_clock::ticks() - ticks > TIMEOUT {
            return Err(Error::AutoNegotiationTimeout); // timeout
        }
    }

    let ssr = phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, SPECIAL_STATUS_REG);
    // auto-negotiation done bit should be set
    assert!(ssr.get_bit(12));
    let (duplex, speed) = match ssr.get_range(2..5) {
        0b001 => (false, Speed::Speed10M), // 10BASE-T half-duplex
        0b101 => (true, Speed::Speed10M), // 10BASE-T full-duplex
        0b010 => (false, Speed::Speed100M), // 100BASE-TX half-duplex
        0b110 => (true, Speed::Speed100M), // 100BASE-TX full-duplex
        other => unreachable!("invalid auto-negotiation value: {:#b}", other),
    };
    Ok(AutoNegotiationResult {
        duplex: duplex,
        speed: speed,
    })
}

fn phy_read(ethernet_mac: &mut EthernetMac, phy_address: u8, register: u8) -> u16 {
    // set the MII address register
    ethernet_mac.macmiiar.update(|r| {
        assert_eq!(r.mb(), false); // assert that MII is not busy

        r.set_pa(phy_address); // set phy address
        r.set_mr(register); // set mii register address
        r.set_mw(false); // MII write operation (false = read)
        r.set_mb(true); // MII busy
    });

    // wait for completion (busy flag cleared)
    while ethernet_mac.macmiiar.read().mb() {}

    // read the value from the MII data register
    ethernet_mac.macmiidr.read().td()
}

fn phy_write(ethernet_mac: &mut EthernetMac, phy_address: u8, register: u8, value: u16) {
    assert_eq!(ethernet_mac.macmiiar.read().mb(), false); // assert that MII is not busy

    // give the value to the MII data register
    let mut macmiidr = ethernet_mac::Macmiidr::default();
    macmiidr.set_td(value);
    ethernet_mac.macmiidr.write(macmiidr);

    // set the MII address register
    ethernet_mac.macmiiar.update(|r| {
        r.set_pa(phy_address); // set phy address
        r.set_mr(register); // set mii register address
        r.set_mw(true); // MII write operation (true = write)
        r.set_mb(true); // MII busy
    });

    // wait for completion (busy flag cleared)
    while ethernet_mac.macmiiar.read().mb() {}
}
