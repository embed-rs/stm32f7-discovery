//! Provides various hardware initialization functions.

use crate::i2c::{self, I2C};
use crate::lcd::{self, Lcd};
use crate::system_clock;
use stm32f7::stm32f7x6::{self as device, FLASH, FMC, LTDC, PWR, RCC, SAI2, SYST};

pub use self::pins::init as pins;

mod pins;

/// Initialize the system clock to the maximum speed of 216MHz.
///
/// This function should be called right at the beginning of the main function.
pub fn init_system_clock_216mhz(rcc: &mut RCC, pwr: &mut PWR, flash: &mut FLASH) {
    // enable power control clock
    rcc.apb1enr.modify(|_, w| w.pwren().enabled());
    rcc.apb1enr.read(); // delay

    // reset HSEON and HSEBYP bits before configuring HSE
    rcc.cr.modify(|_, w| {
        w.hseon().clear_bit();
        w.hsebyp().clear_bit();
        w
    });
    // wait until HSE is disabled
    while rcc.cr.read().hserdy().bit_is_set() {}
    // turn HSE on
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    // wait until HSE is enabled
    while rcc.cr.read().hserdy().bit_is_clear() {}

    // disable main PLL
    rcc.cr.modify(|_, w| w.pllon().clear_bit());
    while rcc.cr.read().pllrdy().bit_is_set() {}

    // Configure the main PLL clock source, multiplication and division factors.
    // HSE is used as clock source. HSE runs at 25 MHz.
    // PLLM = 25: Division factor for the main PLLs (PLL, PLLI2S and PLLSAI) input clock
    // VCO input frequency = PLL input clock frequency / PLLM with 2 ≤ PLLM ≤ 63
    // => VCO input frequency = 25_000 kHz / 25 = 1_000 kHz = 1 MHz
    // PPLM = 432: Main PLL (PLL) multiplication factor for VCO
    // VCO output frequency = VCO input frequency × PLLN with 50 ≤ PLLN ≤ 432
    // => VCO output frequency 1 Mhz * 432 = 432 MHz
    // PPLQ = 0 =^= division factor 2: Main PLL (PLL) division factor for main system clock
    // PLL output clock frequency = VCO frequency / PLLP with PLLP = 2, 4, 6, or 8
    // => PLL output clock frequency = 432 MHz / 2 = 216 MHz
    rcc.pllcfgr.modify(|_, w| {
        w.pllsrc().hse();
        w.pllp().div2();
        unsafe {
            // Frequency = ((TICKS / pllm) * plln) / pllp
            // HSE runs at 25 MHz
            w.pllm().bits(25);
            w.plln().bits(432); // 400 for 200 MHz, 432 for 216 MHz
            w.pllq().bits(9); // 8 for 200 MHz, 9 for 216 MHz
        }
        w
    });
    // enable main PLL
    rcc.cr.modify(|_, w| w.pllon().set_bit());
    while rcc.cr.read().pllrdy().bit_is_clear() {}

    // enable overdrive
    pwr.cr1.modify(|_, w| w.oden().set_bit());
    while pwr.csr1.read().odrdy().bit_is_clear() {}
    // enable overdrive switching
    pwr.cr1.modify(|_, w| w.odswen().set_bit());
    while pwr.csr1.read().odswrdy().bit_is_clear() {}

    // Program the new number of wait states to the LATENCY bits in the FLASH_ACR register
    flash.acr.modify(|_, w| w.latency().bits(5));
    // Check that the new number of wait states is taken into account to access the Flash
    // memory by reading the FLASH_ACR register
    assert_eq!(flash.acr.read().latency().bits(), 5);

    // HCLK Configuration
    // HPRE = system clock not divided: AHB prescaler
    // => AHB clock frequency = system clock / 1 = 216 MHz / 1 = 216 MHz
    rcc.cfgr.modify(|_, w| w.hpre().div1());
    // SYSCLK Configuration
    rcc.cfgr.modify(|_, w| w.sw().pll());
    while !rcc.cfgr.read().sws().is_pll() {}

    // PCLK1 Configuration
    // PPRE1: APB Low-speed prescaler (APB1)
    // => APB low-speed clock frequency = AHB clock / 4 = 216 Mhz / 4 = 54 MHz
    // FIXME: Frequency should not exceed 45 MHz
    rcc.cfgr.modify(|_, w| w.ppre1().div4());
    // PCLK2 Configuration
    // PPRE2: APB high-speed prescaler (APB2)
    // => APB high-speed clock frequency = AHB clock / 2 = 216 Mhz / 2 = 108 MHz
    // FIXME: Frequency should not exceed 90 MHz
    rcc.cfgr.modify(|_, w| w.ppre2().div2());
}

/// Initialize the system clock to the specified frequency.
///
/// Equivalent to [`system_clock::init`](crate::system_clock::init).
pub fn init_systick(frequency: system_clock::Hz, systick: &mut SYST, rcc: &RCC) {
    system_clock::init(frequency, systick, rcc)
}

/// Enable all GPIO ports in the RCC register.
pub fn enable_gpio_ports(rcc: &mut RCC) {
    rcc.ahb1enr.modify(|_, w| {
        w.gpioaen().enabled();
        w.gpioben().enabled();
        w.gpiocen().enabled();
        w.gpioden().enabled();
        w.gpioeen().enabled();
        w.gpiofen().enabled();
        w.gpiogen().enabled();
        w.gpiohen().enabled();
        w.gpioien().enabled();
        w.gpiojen().enabled();
        w.gpioken().enabled();
        w
    });
    // wait till enabled
    loop {
        let ahb1enr = rcc.ahb1enr.read();
        if ahb1enr.gpioaen().is_enabled()
            && ahb1enr.gpioben().is_enabled()
            && ahb1enr.gpiocen().is_enabled()
            && ahb1enr.gpioden().is_enabled()
            && ahb1enr.gpioeen().is_enabled()
            && ahb1enr.gpiofen().is_enabled()
            && ahb1enr.gpiogen().is_enabled()
            && ahb1enr.gpiohen().is_enabled()
            && ahb1enr.gpioien().is_enabled()
            && ahb1enr.gpiojen().is_enabled()
            && ahb1enr.gpioken().is_enabled()
        {
            break;
        }
    }
}

/// Enable the syscfg clock.
pub fn enable_syscfg(rcc: &mut RCC) {
    // enable syscfg clock
    rcc.apb2enr.modify(|_, w| w.syscfgen().set_bit());
    // delay
    let _unused = rcc.apb2enr.read();
}

/// Initializes the SDRAM, which makes more memory accessible.
///
/// This is a prerequisite for using the LCD.
pub fn init_sdram(rcc: &mut RCC, fmc: &mut FMC) {
    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy)]
    enum Bank {
        One,
        Two,
        Both,
    }

    /// When a command is issued, at least one Command Target Bank bit ( CTB1 or CTB2) must be
    /// set otherwise the command will be ignored.
    ///
    /// Note: If two SDRAM banks are used, the Auto-refresh and PALL command must be issued
    /// simultaneously to the two devices with CTB1 and CTB2 bits set otherwise the command will
    /// be ignored.
    ///
    /// Note: If only one SDRAM bank is used and a command is issued with it’s associated CTB bit
    /// set, the other CTB bit of the the unused bank must be kept to 0.
    #[allow(dead_code)]
    #[repr(u8)]
    enum Command {
        Normal = 0b000,
        ClockConfigurationEnable = 0b001,
        PrechargeAllCommand = 0b010,
        AutoRefreshCommand = 0b011,
        LoadModeRegister = 0b100,
        SelfRefreshCommand = 0b101,
        PowerDownCommand = 0b110,
    }

    fn send_fmc_command(
        fmc: &mut FMC,
        bank: Bank,
        command: Command,
        auto_refresh: u8,
        modereg: u16,
    ) {
        assert!(fmc.sdsr.read().busy().bit_is_clear());

        fmc.sdcmr.modify(|_, w| {
            match bank {
                Bank::One => {
                    w.ctb1().set_bit();
                }
                Bank::Two => {
                    w.ctb2().set_bit();
                }
                Bank::Both => {
                    w.ctb1().set_bit();
                    w.ctb2().set_bit();
                }
            };
            unsafe {
                w.mode().bits(command as u8);
                w.nrfs().bits(auto_refresh); // number_of_auto_refresh
                w.mrd().bits(modereg); // mode_register_definition
            }
            w
        });

        while fmc.sdsr.read().busy().bit_is_set() {
            // wait
        }
    }

    // Enable FMC clock
    rcc.ahb3enr.modify(|_, w| w.fmcen().enabled());

    // Reset FMC module
    rcc.ahb3rstr.modify(|_, w| w.fmcrst().reset());
    rcc.ahb3rstr.modify(|_, w| w.fmcrst().clear_bit());

    // SDRAM contol register
    fmc.sdcr1.modify(|_, w| unsafe {
        w.nc().bits(8 - 8); // number_of_column_address_bits
        w.nr().bits(12 - 11); // number_of_row_address_bits
        w.mwid().bits(0b01 /* = 16 */); // data_bus_width
        w.nb().bit(true /* = 4 */); // number_of_internal_banks
        w.cas().bits(2); // cas_latency
        w.wp().bit(false); // write_protection
        w.rburst().bit(false); // burst_read
        w.sdclk().bits(2); // enable_sdram_clock
        w
    });

    // SDRAM timings
    fmc.sdtr1.modify(|_, w| unsafe {
        w.tmrd().bits(2 - 1); // load_mode_register_to_active
        w.txsr().bits(7 - 1); // exit_self_refresh_delay
        w.tras().bits(4 - 1); // self_refresh_time
        w.trc().bits(7 - 1); // row_cycle_delay
        w.twr().bits(2 - 1); // recovery_delay
        w.trp().bits(2 - 1); // row_precharge_delay
        w.trcd().bits(2 - 1); // row_to_column_delay
        w
    });

    let banks = Bank::One;

    // enable clock config
    send_fmc_command(fmc, banks, Command::ClockConfigurationEnable, 1, 0);
    // wait at least 100μs while the sdram powers up
    system_clock::wait_ms(1);

    // Precharge all Command
    send_fmc_command(fmc, banks, Command::PrechargeAllCommand, 1, 0);

    // Set auto refresh
    send_fmc_command(fmc, banks, Command::AutoRefreshCommand, 8, 0);

    // Load the external mode register
    // BURST_LENGTH_1 | BURST_TYPE_SEQUENTIAL | CAS_LATENCY_2 | OPERATING_MODE_STANDARD
    // | WRITEBURST_MODE_SINGLE;
    let mrd = 0x0020 | 0x200;
    send_fmc_command(fmc, banks, Command::LoadModeRegister, 1, mrd);

    // set refresh counter
    fmc.sdrtr.modify(|_, w| unsafe {
        w.count().bits(0x301);
        w.reie().bit(false);
        w
    });

    // test sdram
    use core::ptr;

    let ptr1 = 0xC000_0000 as *mut u32;
    let ptr2 = 0xC053_6170 as *mut u32;
    let ptr3 = 0xC07F_FFFC as *mut u32;

    unsafe {
        ptr::write_volatile(ptr1, 0xcafebabe);
        ptr::write_volatile(ptr2, 0xdeadbeaf);
        ptr::write_volatile(ptr3, 0x0deafbee);
        assert_eq!(ptr::read_volatile(ptr1), 0xcafebabe);
        assert_eq!(ptr::read_volatile(ptr2), 0xdeadbeaf);
        assert_eq!(ptr::read_volatile(ptr3), 0x0deafbee);
    }
}

/// Initializes the LCD.
///
/// This function is equivalent to [`lcd::init`](crate::lcd::init::init).
pub fn init_lcd<'a>(ltdc: &'a mut LTDC, rcc: &mut RCC) -> Lcd<'a> {
    lcd::init(ltdc, rcc)
}

/// Initializes the I2C3 bus.
///
/// This function is equivalent to [`i2c::init`](crate::i2c::init).
pub fn init_i2c_3(i2c: device::I2C3, rcc: &mut RCC) -> I2C<device::I2C3> {
    i2c::init(i2c, rcc)
}

/// Initializes the SAI2 controller.
///
/// Required for audio input.
pub fn init_sai_2(sai: &mut SAI2, rcc: &mut RCC) {
    let audio_frequency = 16000;

    // disable block a and block b
    sai.acr1.modify(|_, w| w.saiaen().clear_bit()); // audio_block_enable
    sai.bcr1.modify(|_, w| w.saiben().clear_bit()); // audio_block_enable
    while sai.acr1.read().saiaen().bit_is_set() {}
    while sai.bcr1.read().saiben().bit_is_set() {}

    // enable sai2 clock
    rcc.apb2enr.modify(|_, w| w.sai2en().set_bit());

    // Disabled All interrupt and clear all the flag
    sai.bim.write(|w| w);
    // Clear all flags
    sai.bclrfr.write(|w| {
        w.lfsdet().set_bit(); // Clear late frame synchronization detection flag
        w.cafsdet().set_bit(); // Clear anticipated frame synchronization detection flag
        w.cnrdy().set_bit(); // Clear codec not ready flag
        w.wckcfg().set_bit(); // Clear wrong clock configuration flag
        w.mutedet().set_bit(); // Clear mute detection flag
        w.ovrudr().set_bit(); // Clear overrun / underrun
        w
    });

    // Flush the fifo
    sai.bcr2.modify(|_, w| w.fflus().set_bit()); // fifo_flush

    // PLL clock is set depending on the AudioFreq (44.1khz vs 48khz groups)

    // I2S clock config
    // PLLI2S_VCO: VCO_344M
    // I2S_CLK(first level) = PLLI2S_VCO/PLLI2SQ = 344/7 = 49.142 Mhz
    // I2S_CLK_x = I2S_CLK(first level)/PLLI2SDIVQ = 49.142/1 = 49.142 Mhz

    // Configure SAI2 Clock source
    rcc.dkcfgr1.modify(|_, w| unsafe { w.sai2sel().bits(0) }); // sai2_clock_source plli2s

    // Disable the PLLI2S
    rcc.cr.modify(|_, w| w.plli2son().clear_bit());
    while rcc.cr.read().plli2srdy().bit_is_set() {}

    // Configure the PLLI2S division factors
    // PLLI2S_VCO Input  = PLL_SOURCE/PLLM
    // PLLI2S_VCO Output = PLLI2S_VCO Input * PLLI2SN
    // SAI_CLK(first level) = PLLI2S_VCO Output/PLLI2SQ
    rcc.plli2scfgr.modify(|_, w| unsafe {
        w.plli2sn().bits(344);
        w.plli2sq().bits(7);
        w
    });

    // SAI_CLK_x = SAI_CLK(first level)/PLLI2SDIVQ
    rcc.dkcfgr1
        .modify(|_, w| unsafe { w.plli2sdiv().bits(1 - 1) });

    // Enable the PLLI2S
    rcc.cr.modify(|_, w| w.plli2son().set_bit());
    while rcc.cr.read().plli2srdy().bit_is_clear() {}

    // configure sai registers

    // disable synchronization outputs
    sai.gcr.modify(|_, w| unsafe { w.syncout().bits(0) }); // NoSyncOutput

    // Initialize SAI2 block A in MASTER RX

    // configure cr1
    let mckdiv = {
        // Configure Master Clock using the following formula :
        // MCLK_x = SAI_CK_x / (MCKDIV[3:0] * 2) with MCLK_x = 256 * FS
        // FS = SAI_CK_x / (MCKDIV[3:0] * 2) * 256
        // MCKDIV[3:0] = SAI_CK_x / FS * 512

        // Get SAI clock source based on Source clock selection from RCC
        let freq = {
            // Configure the PLLSAI division factor
            // PLLSAI_VCO Input  = PLL_SOURCE/PLLM
            // In Case the PLL Source is HSE (External Clock)
            let vcoinput = 25000000 / u32::from(rcc.pllcfgr.read().pllm().bits());

            // PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAIN
            // SAI_CLK(first level) = PLLSAI_VCO Output/PLLSAIQ
            let tmpreg = u32::from(rcc.pllsaicfgr.read().pllsaiq().bits());
            let frequency = (vcoinput * u32::from(rcc.pllsaicfgr.read().pllsain().bits())) / tmpreg;

            // SAI_CLK_x = SAI_CLK(first level)/PLLSAIDIVQ
            let tmpreg = u32::from(rcc.dkcfgr1.read().pllsaidivq().bits()) + 1;
            frequency / tmpreg
        };

        // (saiclocksource x 10) to keep Significant digits
        let tmpclock = (freq * 10) / (audio_frequency * 512);

        let mckdiv = tmpclock / 10;

        // Round result to the nearest integer
        if (tmpclock % 10) > 8 {
            mckdiv + 1
        } else {
            mckdiv
        }
    };

    sai.acr1.write(|w| unsafe {
        w.mode().bits(0b01); // MasterReceiver
        w.prtcfg().bits(0b00); // protocol free
        w.ds().bits(0b100); // data_size 16 bits
        w.lsbfirst().clear_bit();
        w.ckstr().set_bit(); // clock_strobing_edge
        w.syncen().bits(0b00); // synchronization asynchronous
        w.mono().clear_bit();
        w.out_dri().set_bit(); // output_drive
        w.nodiv().clear_bit(); // no_divider
        w.mcjdiv().bits(mckdiv as u8); // master_clock_divider8
        w
    });

    // configure cr2
    sai.acr2.write(|w| unsafe {
        w.fth().bits(0b001); // fifo_threshold QuarterFifo
        w.tris().clear_bit(); // tristate_management
        w.comp().bits(0b00); // companding_mode None
        w
    });

    // configure frame
    sai.afrcr.write(|w| unsafe {
        w.frl().bits(64 - 1); // frame_length
        w.fsall().bits(32 - 1); // sync_active_level_length
        w.fsdef().set_bit(); // frame_sync_definition
        w.fspol().clear_bit(); // frame_sync_polarity
        w.fsoff().set_bit(); // frame_sync_offset
        w
    });

    // configure slot
    sai.aslotr.write(|w| unsafe {
        w.fboff().bits(0); // first_bit_offset
        w.slotsz().bits(0b00); // slot_size DataSize
        w.nbslot().bits(4 - 1); // number_of_slots
        w.sloten().bits(1 << 1 | 1 << 3); // enable_slots
        w
    });

    // Initialize SAI2 block B in SLAVE RX synchronous from SAI2 block A

    // configure cr1
    sai.bcr1.write(|w| unsafe {
        w.mode().bits(0b11); // SlaveReceiver
        w.prtcfg().bits(0b00); // protocol free
        w.ds().bits(0b100); // data_size 16 bits
        w.lsbfirst().clear_bit();
        w.ckstr().set_bit(); // clock_strobing_edge
        w.syncen().bits(0b01); // synchronization SynchronousWithOtherSubBlock
        w.mono().clear_bit();
        w.out_dri().set_bit(); // output_drive
        w.nodiv().clear_bit(); // no_divider
        w.mcjdiv().bits(mckdiv as u8); // master_clock_divider8
        w
    });

    // configure cr2
    sai.bcr2.write(|w| unsafe {
        w.fth().bits(0b001); // fifo_threshold QuarterFifo
        w.tris().clear_bit(); // tristate_management
        w.comp().bits(0b00); // companding_mode None
        w
    });

    // configure frame
    sai.bfrcr.write(|w| {
        unsafe {
            w.frl().bits(64 - 1); // frame_length
            w.fsall().bits(32 - 1);
        } // sync_active_level_length
        w.fsdef().set_bit(); // frame_sync_definition
        w.fspol().clear_bit(); // frame_sync_polarity
        w.fsoff().set_bit(); // frame_sync_offset
        w
    });

    // configure slot
    sai.bslotr.write(|w| unsafe {
        w.fboff().bits(0); // first_bit_offset
        w.slotsz().bits(0b00); // slot_size DataSize
        w.nbslot().bits(4 - 1); // number_of_slots
        w.sloten().bits(1 << 1 | 1 << 3); // enable_slots
        w
    });

    // enable receive interrupts
    /*
    sai.aim.modify(|_, w| {
        w.ovrudrie().set_bit();
        w.freqie().set_bit();
        w.wckcfg().set_bit();
        w
    });
    sai.bim.modify(|_, w| {
        w.ovrudrie().set_bit();
        w.freqie().set_bit();
        w.afsdetie().set_bit();
        w.lfsdetie().set_bit();
        w
    });
    */

    // Enable SAI peripheral block a to generate MCLK
    sai.acr1.modify(|_, w| w.saiaen().set_bit()); // audio_block_enable

    // Enable SAI peripheral block b
    sai.bcr1.modify(|_, w| w.saiben().set_bit()); // audio_block_enable
}

const WM8994_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0011010);

/// Initializes the WM8994 audio controller.
///
/// Required for audio input.
pub fn init_wm8994(i2c_3: &mut i2c::I2C<device::I2C3>) -> Result<(), i2c::Error> {
    i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {
        // read and check device family ID
        assert_eq!(conn.read(0).ok(), Some(0x8994));
        // reset device
        conn.write(0, 0)?;

        // wm8994 Errata Work-Arounds
        conn.write(0x102, 0x0003)?;
        conn.write(0x817, 0x0000)?;
        conn.write(0x102, 0x0000)?;

        // Enable VMID soft start (fast), Start-up Bias Current Enabled
        conn.write(0x39, 0x006C)?;

        // Enable bias generator, Enable VMID
        conn.write(0x01, 0x0003)?;

        system_clock::wait_ms(50);

        // INPUT_DEVICE_DIGITAL_MICROPHONE_2 :

        // Enable AIF1ADC2 (Left), Enable AIF1ADC2 (Right)
        // Enable DMICDAT2 (Left), Enable DMICDAT2 (Right)
        // Enable Left ADC, Enable Right ADC
        conn.write(0x04, 0x0C30)?;

        // Enable AIF1 DRC2 Signal Detect & DRC in AIF1ADC2 Left/Right Timeslot 1
        conn.write(0x450, 0x00DB)?;

        // Disable IN1L, IN1R, IN2L, IN2R, Enable Thermal sensor & shutdown
        conn.write(0x02, 0x6000)?;

        // Enable the DMIC2(Left) to AIF1 Timeslot 1 (Left) mixer path
        conn.write(0x608, 0x0002)?;

        // Enable the DMIC2(Right) to AIF1 Timeslot 1 (Right) mixer path
        conn.write(0x609, 0x0002)?;

        // GPIO1 pin configuration GP1_DIR = output, GP1_FN = AIF1 DRC2 signal detect
        conn.write(0x700, 0x000E)?;

        // Clock Configurations

        // AIF1 Sample Rate = 16 (KHz), ratio=256
        conn.write(0x210, 0x0033)?;

        // AIF1 Word Length = 16-bits, AIF1 Format = I2S (Default Register Value)
        conn.write(0x300, 0x4010)?;

        // slave mode
        conn.write(0x302, 0x0000)?;

        // Enable the DSP processing clock for AIF1, Enable the core clock
        conn.write(0x208, 0x000A)?;

        // Enable AIF1 Clock, AIF1 Clock Source = MCLK1 pin
        conn.write(0x200, 0x0001)?;

        // Enable Microphone bias 1 generator, Enable VMID
        conn.write(0x01, 0x0013)?;

        // ADC oversample enable
        conn.write(0x620, 0x0002)?;

        // AIF ADC2 HPF enable, HPF cut = voice mode 1 fc=127Hz at fs=8kHz
        conn.write(0x411, 0x3800)?;

        // set volume

        let convertedvol = 239; // 100(+17.625dB)

        // Left AIF1 ADC1 volume
        conn.write(0x400, convertedvol | 0x100)?;

        // Right AIF1 ADC1 volume
        conn.write(0x401, convertedvol | 0x100)?;

        // Left AIF1 ADC2 volume
        conn.write(0x404, convertedvol | 0x100)?;

        // Right AIF1 ADC2 volume
        conn.write(0x405, convertedvol | 0x100)?;

        Ok(())
    })
}
