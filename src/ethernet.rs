use board::rcc::Rcc;
use board::syscfg::Syscfg;
use board::ethernet_dma::EthernetDma;
use board::ethernet_mac::EthernetMac;
use system_clock;

pub fn init(rcc: &mut Rcc,
            syscfg: &mut Syscfg,
            ethernet_mac: &mut EthernetMac,
            ethernet_dma: &mut EthernetDma) {

    // enable syscfg clock
    rcc.apb2enr.update(|r| r.set_syscfgen(true));
    // delay
    let _unused = rcc.apb2enr.read();

    // select MII mode
    syscfg.pmc.update(|r| r.set_mii_rmii_sel(true)); // false = MII, true = RMII

    // ethernet software reset in DMA bus mode register
    ethernet_dma.dmabmr.update(|r| r.set_sr(true)); // set software reset bit
    while ethernet_dma.dmabmr.read().sr() {} // wait for auto clear

    // MAC init: set clock range in MAC MII address register
    let clock_range = match system_clock::get_frequency() {
        f if f >= 150000000 => 0b100, // 150-168 MHz HCLK/102
        _ => panic!("unsupported"),
    };
    ethernet_mac.macmiiar.update(|r| r.set_cr(clock_range));

    // init PHY
    phy::init(ethernet_mac);

    // MAC config
    // configuration register
    ethernet_mac.maccr.update(|r| {
        r.set_fes(true); // fast ethernet speed (false = 10Mbit/s, true = 100Mbit/s)
        r.set_dm(true); // duplex mode

        r.set_lm(false); // loopback mode
        r.set_apcs(true); // automatic pad/CRC stripping (only if length <= 1500 bytes)
        r.set_cstf(true); // CRC stripping for Type frames
        r.set_ifg(0); // inter frame gap (0 = 96bit)
        r.set_csd(false); // carrier sense disable

        // When set, this bit enables IPv4 checksum checking for received frame payloads'
        // TCP/UDP/ICMP headers. When this bit is reset, the checksum offload function in the
        // receiver is disabled.
        r.set_ipco(true); // IPv4 checksum offload

        // When this bit is set, the MAC disables the watchdog timer on the receiver, and can
        // receive frames of up to 16 384 bytes. When this bit is reset, the MAC allows no more
        // than 2 048 bytes of the frame being received and cuts off any bytes received after that.
        r.set_wd(false); // watchdog disable

        // When this bit is set, the MAC disables the jabber timer on the transmitter, and can
        // transfer frames of up to 16 384 bytes. When this bit is reset, the MAC cuts off the
        // transmitter if the application sends out more than 2 048 bytes of data during
        // transmission.
        r.set_jd(false); // jabber disable
    });

    // frame filter register
    ethernet_mac.macffr.update(|r| {
        r.set_pm(false); // Promiscuous mode
        r.set_ra(false); // receive all (ignoring address filters)
        r.set_hpf(false); // Hash or perfect filter
        r.set_saf(false); // Source address filter
        r.set_saif(false); // Source address inverse filtering
        r.set_daif(false); // Destination address inverse filtering
        r.set_bfd(false); // broadcast frames disable
        r.set_ram(false); // pass all multicast
        r.set_hu(false); // hash unicast
        r.set_hm(false); // hash multicast

        // TODO FIXME: reference manual and generated code are different
        r.set_pcf(false); // pass control frames
    });

    // hash table low/high
    // TODO


}

mod phy {
    use board::ethernet_mac::{self, EthernetMac};
    use bit_field::BitField;
    use system_clock;

    const LAN8742A_PHY_ADDRESS: u8 = 0;

    const BASIC_CONTROL_REG: u8 = 0;
    const BASIC_STATUS_REG: u8 = 1; // basic status register

    const PHY_RESET: u16 = 1 << 15;
    const AUTONEGOTIATION_ENABLE: u16 = 1 << 12;

    pub fn init(ethernet_mac: &mut EthernetMac) {
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
        while !phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(2) {}

        // enable auto-negotiation
        phy_write(ethernet_mac,
                  LAN8742A_PHY_ADDRESS,
                  BASIC_CONTROL_REG,
                  AUTONEGOTIATION_ENABLE);
        // wait until auto-negotiation complete bit is set
        while phy_read(ethernet_mac, LAN8742A_PHY_ADDRESS, BASIC_STATUS_REG).get_bit(5) {}
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
}
