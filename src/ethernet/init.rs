use board::rcc::Rcc;
use board::syscfg::Syscfg;
use board::ethernet_dma::EthernetDma;
use board::ethernet_mac::EthernetMac;
use embedded::interfaces::gpio::Gpio;
use system_clock;
use super::phy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    PhyError(phy::Error),
}

impl From<phy::Error> for Error {
    fn from(err: phy::Error) -> Error {
        Error::PhyError(err)
    }
}

pub fn init(
    rcc: &mut Rcc,
    syscfg: &mut Syscfg,
    gpio: &mut Gpio,
    ethernet_mac: &mut EthernetMac,
    ethernet_dma: &mut EthernetDma,
) -> Result<(), Error> {
    // TODO delay after writes?

    // enable syscfg clock
    rcc.apb2enr.update(|r| r.set_syscfgen(true));
    // delay
    let _unused = rcc.apb2enr.read();

    init_pins(gpio);

    // TODO enable interrupt

    // enable ethernet clocks
    rcc.ahb1enr.update(|r| {
        r.set_ethmacen(true); // ethernet mac clock enable
        r.set_ethmactxen(true); // ethernet mac transmission clock enable
        r.set_ethmacrxen(true); // ethernet mac reception clock enable
    });

    // select MII or RMII mode
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
    let auto_neg_result = phy::init(ethernet_mac)?;
    assert!(auto_neg_result.duplex);
    assert_eq!(auto_neg_result.speed, phy::Speed::Speed100M);

    // MAC config
    // configuration register
    ethernet_mac.maccr.update(|r| {
        // fast ethernet speed (false = 10Mbit/s, true = 100Mbit/s)
        r.set_fes(match auto_neg_result.speed {
            phy::Speed::Speed100M => true,
            phy::Speed::Speed10M => false,
        });
        // duplex mode
        r.set_dm(auto_neg_result.duplex);

        r.set_lm(false); // loopback mode
        r.set_apcs(true); // automatic pad/CRC stripping (only if length <= 1500 bytes)
        r.set_cstf(true); // CRC stripping for Type frames
        r.set_ifg(0); // inter frame gap (0 = 96bit)
        r.set_csd(false); // carrier sense disable

        // When set, this bit enables IPv4 checksum checking for received frame payloads'
        // TCP/UDP/ICMP headers. When this bit is reset, the checksum offload function in the
        // receiver is disabled.
        r.set_ipco(false); // IPv4 checksum offload

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

    // hash table low/high register
    ethernet_mac.machtlr.update(|r| r.set_htl(0));
    ethernet_mac.machthr.update(|r| r.set_hth(0));

    // flow control register
    ethernet_mac.macfcr.update(|r| {
        r.set_pt(0); // pause time
        r.set_zqpd(true); // zero-quanta post disable bit
        r.set_plt(0b00); // pause low threshold (0b00 == "Pause time minus 4 slot times")
        r.set_upfd(false); // unicast pause frame detect
        r.set_rfce(false); // receive flow control enable
        r.set_tfce(false); // transmit flow control enable
        r.set_fcb(false); // flow control busy/back pressure activate
    });

    // VLAN tag register
    ethernet_mac.macvlantr.update(|r| {
        r.set_vlanti(0); // VLAN tag identifier (for receive frames)
        r.set_vlantc(false); // 12-bit VLAN tag comparison (false == 16 bit comparison)
    });

    // DMA init
    // operation mode register
    ethernet_dma.dmaomr.update(|r| {
        r.set_sr(false); // start/stop receive (false = stopped)
        r.set_osf(true); // operate on second frame
        r.set_rtc(0b00); // receive threshold control (0b00 = 64 bytes)
        r.set_fugf(false); // forward undersized good frames
        r.set_fef(false); // forward error frames
        r.set_st(false); // start/stop transmission (false = stopped)
        r.set_ttc(0b000); // transmit threshold control (0b000 = 64 bytes)
        r.set_ftf(false); // flush transmit FIFO
        r.set_tsf(true); // transmit store and forward
        r.set_dfrf(false); // disable flushing of received frames
        r.set_rsf(true); // receive store and forward
        r.set_dtcefd(false); // dropping of TCP/IP checksum error frames disable
    });

    // bus mode register
    ethernet_dma.dmabmr.update(|r| {
        r.set_aab(true); // address-aligned beats
        r.set_fb(true); // fixed burst
        r.set_rdp(32); // Rx DMA Programmable burst length
        r.set_pbl(32); // TX DMA Programmable burst length
        r.set_edfe(false); // Enhanced descriptor format enable
        r.set_dsl(0); // Descriptor skip length
        r.set_da(false); // DMA Arbitration (false = Round-robin with Rx:Tx priority given in `pm`)
        r.set_usp(true); // Use separate PBL
    });

    // interrupt enable register
    ethernet_dma.dmaier.update(|r| {
        r.set_nise(true); // Normal interrupt summary enable
        r.set_rie(true); // Receive interrupt enable
    });

    // Initialize MAC address in ethernet MAC
    ethernet_mac.maca0hr.update(|r| {
        r.set_maca0h(0 << 8 | 0); // high register
    });
    ethernet_mac.maca0lr.update(|r| {
        r.set_maca0l(0 << 24 | 0 << 16 | 0 << 8 | 2); // low register
    });

    Ok(())
}

pub fn start(ethernet_mac: &mut EthernetMac, ethernet_dma: &mut EthernetDma) {
    // enable MAC transmission and reception
    ethernet_mac.maccr.update(|r| {
        r.set_te(true);
        r.set_re(true);
    });

    // flush transmit FIFO and enable DMA transmission/reception
    ethernet_dma.dmaomr.update(|r| {
        r.set_ftf(true);
        r.set_st(true);
        r.set_sr(true);
    });
}

pub fn init_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::{AlternateFunction, OutputSpeed, OutputType, Resistor};

    // RMII pins
    let ref_clk = (PortA, Pin1);
    let mdio = (PortA, Pin2);
    let mdc = (PortC, Pin1);
    let crsdv = (PortA, Pin7);
    let rxd0 = (PortC, Pin4);
    let rxd1 = (PortC, Pin5);
    let tx_en = (PortG, Pin11);
    let txd0 = (PortG, Pin13);
    let txd1 = (PortG, Pin14);

    let pins = [txd0, txd1, tx_en, rxd0, rxd1, crsdv, mdc, mdio, ref_clk];
    gpio.to_alternate_function_all(
        &pins,
        AlternateFunction::AF11,
        OutputType::PushPull,
        OutputSpeed::High,
        Resistor::NoPull,
    ).unwrap();
}
