use crate::system_clock;
use bit_field::BitField;
use stm32f7::stm32f7x6::ETHERNET_MAC;

const LAN8742A_PHY_ADDRESS: u8 = 0;

const BASIC_CONTROL_REG: u8 = 0;
const BASIC_STATUS_REG: u8 = 1; // basic status register
const SPECIAL_STATUS_REG: u8 = 31; // special status register

const PHY_RESET: u16 = 1 << 15;
const AUTONEGOTIATION_ENABLE: u16 = 1 << 12;
const AUTONEGOTIATION_RESTART: u16 = 1 << 9;

const TIMEOUT_MS: usize = 5000;

/// Errors that can happen during initialization of the PHY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Timeout while waiting for ethernet link.
    LinkTimeout,
    /// Timeout while waiting for auto negotiation.
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

pub fn init(ethernet_mac: &mut ETHERNET_MAC) -> Result<AutoNegotiationResult, Error> {
    // reset PHY
    phy_write(
        ethernet_mac,
        LAN8742A_PHY_ADDRESS,
        BASIC_CONTROL_REG,
        PHY_RESET,
    );
    // wait 0.5s
    system_clock::wait_ms(500);
    // wait for reset bit auto clear
    while phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_CONTROL_REG) & PHY_RESET != 0 {}

    // wait for link bit
    print!("wait for ethernet link");
    let timeout_ticks = system_clock::ms_to_ticks(TIMEOUT_MS);
    let ticks = system_clock::ticks();
    while !phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(2) {
        if system_clock::ticks() - ticks > timeout_ticks {
            println!(" [TIMEOUT]");
            return Err(Error::LinkTimeout); // timeout
        }
    }
    println!(" [OK]");

    // enable auto-negotiation
    phy_write(
        ethernet_mac,
        LAN8742A_PHY_ADDRESS,
        BASIC_CONTROL_REG,
        AUTONEGOTIATION_ENABLE | AUTONEGOTIATION_RESTART,
    );

    // wait until auto-negotiation complete bit is set
    print!("wait for auto negotiation of ethernet speed");
    let ticks = system_clock::ticks();
    while !phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(5) {
        if system_clock::ticks() - ticks > timeout_ticks {
            println!(" [TIMEOUT]");
            return Err(Error::AutoNegotiationTimeout); // timeout
        }
    }
    println!(" [OK]");

    let ssr = phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, SPECIAL_STATUS_REG);
    // auto-negotiation done bit should be set
    assert!(ssr.get_bit(12));
    let (duplex, speed) = match ssr.get_bits(2..5) {
        0b001 => (false, Speed::Speed10M),  // 10BASE-T half-duplex
        0b101 => (true, Speed::Speed10M),   // 10BASE-T full-duplex
        0b010 => (false, Speed::Speed100M), // 100BASE-TX half-duplex
        0b110 => (true, Speed::Speed100M),  // 100BASE-TX full-duplex
        other => unreachable!("invalid auto-negotiation value: {:#b}", other),
    };
    Ok(AutoNegotiationResult {
        duplex: duplex,
        speed: speed,
    })
}

fn phy_read(ethernet_mac: &mut ETHERNET_MAC, phy_address: u8, register: u8) -> u16 {
    // set the MII address register
    ethernet_mac.macmiiar.modify(|r, w| {
        assert!(!r.mb().is_busy()); // assert that MII is not busy

        w.pa().bits(phy_address); // set phy address
        w.mr().bits(register); // set mii register address
        w.mw().read(); // MII write operation (false = read)
        w.mb().busy(); // MII busy
        w
    });

    // wait for completion (busy flag cleared)
    while ethernet_mac.macmiiar.read().mb().is_busy() {}

    // read the value from the MII data register
    ethernet_mac.macmiidr.read().md().bits()
}

fn phy_write(ethernet_mac: &mut ETHERNET_MAC, phy_address: u8, register: u8, value: u16) {
    assert!(!ethernet_mac.macmiiar.read().mb().is_busy()); // assert that MII is not busy

    // give the value to the MII data register
    ethernet_mac.macmiidr.write(|w| w.md().bits(value));

    // set the MII address register
    ethernet_mac.macmiiar.modify(|_, w| {
        w.pa().bits(phy_address); // set phy address
        w.mr().bits(register); // set mii register address
        w.mw().write(); // MII write operation (true = write)
        w.mb().busy(); // MII busy
        w
    });

    // wait for completion (busy flag cleared)
    while ethernet_mac.macmiiar.read().mb().is_busy() {}
}
