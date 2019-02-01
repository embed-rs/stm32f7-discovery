pub use phy::Error as PhyError;

use super::phy;
use crate::system_clock;
use stm32f7::stm32f7x6::{ETHERNET_DMA, ETHERNET_MAC, RCC, SYSCFG};

pub fn init(
    rcc: &mut RCC,
    syscfg: &mut SYSCFG,
    ethernet_mac: &mut ETHERNET_MAC,
    ethernet_dma: &mut ETHERNET_DMA,
) -> Result<(), PhyError> {
    // TODO delay after writes?

    // enable syscfg clock
    rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());
    // delay
    let _unused = rcc.apb2enr.read();

    // TODO enable interrupt

    // enable ethernet clocks
    rcc.ahb1enr.modify(|_, w| {
        w.ethmacen().set_bit(); // ethernet mac clock enable
        w.ethmactxen().set_bit(); // ethernet mac transmission clock enable
        w.ethmacrxen().set_bit(); // ethernet mac reception clock enable
        w
    });

    // select MII or RMII mode
    syscfg.pmc.modify(|_, w| w.mii_rmii_sel().set_bit()); // false = MII, true = RMII

    // ethernet software reset in DMA bus mode register
    ethernet_dma.dmabmr.modify(|_, w| w.sr().set_bit()); // set software reset bit
    while ethernet_dma.dmabmr.read().sr().bit_is_set() {} // wait for auto clear

    // MAC init: set clock range in MAC MII address register
    match system_clock::system_clock_speed() {
        f if f.0 >= 150000000 => {
            ethernet_mac.macmiiar.modify(|_, w| w.cr().cr_150_168()); // 150-168 MHz HCLK/102
        }
        _ => panic!("unsupported"),
    };

    // init PHY
    let auto_neg_result = phy::init(ethernet_mac)?;
    assert!(auto_neg_result.duplex);
    assert_eq!(auto_neg_result.speed, phy::Speed::Speed100M);

    // MAC config
    // configuration register
    ethernet_mac.maccr.modify(|_, w| {
        // fast ethernet speed (false = 10Mbit/s, true = 100Mbit/s)
        match auto_neg_result.speed {
            phy::Speed::Speed100M => w.fes().fes100(),
            phy::Speed::Speed10M => w.fes().fes10(),
        };
        // duplex mode
        if auto_neg_result.duplex {
            w.dm().full_duplex();
        } else {
            w.dm().half_duplex();
        }

        w.lm().normal(); // loopback mode
        w.apcs().strip(); // automatic pad/CRC stripping (only if length <= 1500 bytes)
        w.cstf().enabled(); // CRC stripping for Type frames
        w.ifg().ifg96(); // inter frame gap 96bit
        w.csd().disabled(); // carrier sense disable

        // When set, this bit enables IPv4 checksum checking for received frame payloads'
        // TCP/UDP/ICMP headers. When this bit is reset, the checksum offload function in the
        // receiver is disabled.
        w.ipco().disabled(); // IPv4 checksum offload

        // When this bit is set, the MAC disables the watchdog timer on the receiver, and can
        // receive frames of up to 16 384 bytes. When this bit is reset, the MAC allows no more
        // than 2 048 bytes of the frame being received and cuts off any bytes received after that.
        w.wd().enabled(); // watchdog enabled

        // When this bit is set, the MAC disables the jabber timer on the transmitter, and can
        // transfer frames of up to 16 384 bytes. When this bit is reset, the MAC cuts off the
        // transmitter if the application sends out more than 2 048 bytes of data during
        // transmission.
        w.jd().enabled(); // jabber enabled

        w
    });

    // frame filter register
    ethernet_mac.macffr.modify(|_, w| {
        w.pm().disabled(); // Promiscuous mode
        w.ra().disabled(); // receive all (ignoring address filters)
        w.hpf().hash_only(); // Hash or perfect filter
        w.saf().disabled(); // Source address filter
        w.saif().normal(); // Source address inverse filtering
        w.daif().normal(); // Destination address inverse filtering
        w.bfd().disabled(); // broadcast frames disable
        w.ram().disabled(); // pass all multicast
        w.hu().perfect(); // hash unicast
        w.hm().perfect(); // hash multicast
        w.pcf().prevent_all(); // pass control frames
        w
    });

    // hash table low/high register
    ethernet_mac.machtlr.modify(|_, w| w.htl().bits(0));
    ethernet_mac.machthr.modify(|_, w| w.hth().bits(0));

    // flow control register
    ethernet_mac.macfcr.modify(|_, w| {
        w.pt().bits(0); // pause time
        w.zqpd().set_bit(); // zero-quanta post disable bit
        w.plt().plt4(); // pause low threshold (plt4 == "Pause time minus 4 slot times")
        w.upfd().disabled(); // unicast pause frame detect
        w.rfce().disabled(); // receive flow control enable
        w.tfce().disabled(); // transmit flow control enable
        w.fcb().disable_back_pressure(); // flow control busy/back pressure activate

        w
    });

    // VLAN tag register
    ethernet_mac.macvlantr.modify(|_, w| {
        w.vlanti().bits(0); // VLAN tag identifier (for receive frames)
        w.vlantc().vlantc16(); // 12-bit VLAN tag comparison (false == 16 bit comparison)

        w
    });

    // DMA init
    // operation mode register
    ethernet_dma.dmaomr.modify(|_, w| {
        w.sr().stopped(); // start/stop receive (false = stopped)
        w.osf().set_bit(); // operate on second frame
        w.rtc().rtc64(); // receive threshold control (rtc64 = 64 bytes)
        w.fugf().drop(); // forward undersized good frames
        w.fef().drop(); // forward error frames
        w.st().stopped(); // start/stop transmission (false = stopped)
        w.ttc().ttc64(); // transmit threshold control (ttc64 = 64 bytes)
        w.ftf().clear_bit(); // flush transmit FIFO
        w.tsf().store_forward(); // transmit store and forward
        w.dfrf().clear_bit(); // disable flushing of received frames
        w.rsf().store_forward(); // receive store and forward
        w.dtcefd().enabled(); // dropping of TCP/IP checksum error frames disable

        w
    });

    // bus mode register
    ethernet_dma.dmabmr.modify(|_, w| {
        w.aab().aligned(); // address-aligned beats
        w.fb().fixed(); // fixed burst
        w.rdp().rdp32(); // Rx DMA Programmable burst length
        w.pbl().pbl32(); // TX DMA Programmable burst length
        w.edfe().disabled(); // Enhanced descriptor format enable
        w.dsl().bits(0); // Descriptor skip length
        w.da().round_robin(); // DMA Arbitration (false = Round-robin with Rx:Tx priority given in `pm`)
        w.usp().separate(); // Use separate PBL

        w
    });

    // interrupt enable register
    ethernet_dma.dmaier.modify(|_, w| {
        w.nise().set_bit(); // Normal interrupt summary enable
        w.rie().set_bit(); // Receive interrupt enable
        w
    });

    // Initialize MAC address in ethernet MAC
    ethernet_mac.maca0hr.modify(|_, w| {
        w.maca0h().bits(0 << 8 | 0) // high register
    });
    ethernet_mac.maca0lr.modify(|_, w| {
        w.maca0l().bits(0 << 24 | 0 << 16 | 0 << 8 | 2) // low register
    });

    Ok(())
}

pub fn start(ethernet_mac: &mut ETHERNET_MAC, ethernet_dma: &mut ETHERNET_DMA) {
    // enable MAC transmission and reception
    ethernet_mac.maccr.modify(|_, w| {
        w.te().set_bit();
        w.re().set_bit();
        w
    });

    // flush transmit FIFO and enable DMA transmission/reception
    ethernet_dma.dmaomr.modify(|_, w| {
        w.ftf().set_bit();
        w.st().set_bit();
        w.sr().set_bit();
        w
    });
}
