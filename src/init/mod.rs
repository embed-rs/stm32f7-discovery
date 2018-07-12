use core::convert::TryFrom;
use stm32f7::stm32f7x6::{FLASH, FMC, LTDC, PWR, RCC, SYST};
use system_clock;

pub use self::pins::init as pins;

mod pins;

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

pub fn init_systick(Hz(frequency): Hz, systick: &mut SYST, rcc: &RCC) {
    use cortex_m::peripheral::syst::SystClkSource;
    use stm32f7::stm32f7x6::rcc::pllcfgr::PLLPR;

    let pll_cfgr = rcc.pllcfgr.read();
    let pllm = u64::from(pll_cfgr.pllm().bits());
    let plln = u64::from(pll_cfgr.plln().bits());
    let pllp = match pll_cfgr.pllp() {
        PLLPR::DIV2 => 2,
        PLLPR::DIV4 => 4,
        PLLPR::DIV6 => 6,
        PLLPR::DIV8 => 8,
    };

    let system_clock_speed = (((25 * 1000 * 1000) / pllm) * plln) / pllp; // HSE runs at 25 MHz
    let reload_ticks = u32::try_from(system_clock_speed / frequency).unwrap();

    // SysTick Reload Value Register = ((25000/25) * 432) / 2 - 1 = 215_999
    // => SysTick interrupt tiggers every 1 ms
    systick.set_clock_source(SystClkSource::Core);
    systick.set_reload(reload_ticks - 1);
    systick.clear_current();
    systick.enable_counter();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Hz(pub u64);

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
}

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
    system_clock::wait(1);

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

pub fn init_lcd(ltdc: &mut LTDC, rcc: &mut RCC) {
    use lcd::{self, LAYER_1_START, LAYER_2_START};
    const HEIGHT: u16 = lcd::HEIGHT as u16;
    const WIDTH: u16 = lcd::WIDTH as u16;
    const LAYER_1_OCTETS_PER_PIXEL: u16 = lcd::LAYER_1_OCTETS_PER_PIXEL as u16;
    const LAYER_2_OCTETS_PER_PIXEL: u16 = lcd::LAYER_2_OCTETS_PER_PIXEL as u16;

    // enable LTDC and DMA2D clocks
    rcc.ahb1enr.modify(|_, w| w.dma2den().enabled());
    rcc.apb2enr.modify(|_, w| w.ltdcen().enabled());

    // disable LTDC
    ltdc.gcr.modify(|_, w| w.ltdcen().clear_bit());

    // disable PLLSAI clock
    rcc.cr.modify(|_, w| w.pllsaion().clear_bit());
    while rcc.cr.read().pllsairdy().bit_is_set() {}

    rcc.pllsaicfgr.modify(|_, w| unsafe {
        w.pllsain().bits(192);
        w.pllsair().bits(5);
        w
    });

    // set division factor for LCD_CLK
    rcc.dkcfgr1.modify(|_, w| unsafe {
        w.pllsaidivr().bits(0b01 /* = 4 */)
    });

    // enable PLLSAI clock
    rcc.cr.modify(|_, w| w.pllsaion().set_bit());
    while rcc.cr.read().pllsairdy().bit_is_clear() {}

    // configure the HS, VS, DE and PC polarity
    ltdc.gcr.modify(|_, w| {
        w.pcpol().bit(false);
        w.depol().bit(false);
        w.hspol().bit(false);
        w.vspol().bit(false);
        w
    });

    // set synchronization size
    ltdc.sscr.modify(|_, w| unsafe {
        w.hsw().bits(41 - 1); // horizontal_sync_width
        w.vsh().bits(10 - 1); // vertical_sync_height
        w
    });

    // set accumulated back porch
    ltdc.bpcr.modify(|_, w| unsafe {
        w.ahbp().bits(41 + 13 - 1); // accumulated_horizontal_back_porch
        w.avbp().bits(10 + 2 - 1); // accumulated_vertical_back_porch
        w
    });

    // set accumulated active width
    ltdc.awcr.modify(|_, w| unsafe {
        w.aav().bits(WIDTH + 41 + 13 - 1); // accumulated_active_width
        w.aah().bits(HEIGHT + 10 + 2 - 1); // accumulated_active_height
        w
    });

    // set total width
    ltdc.twcr.modify(|_, w| unsafe {
        w.totalw().bits(WIDTH + 41 + 13 + 32 - 1); // total_width
        w.totalh().bits(HEIGHT + 10 + 2 + 2 - 1); // total_height
        w
    });

    // set background color
    ltdc.bccr.modify(|_, w| unsafe { w.bc().bits(0x0000ff) }); // background_color blue

    // enable the transfer error interrupt and the FIFO underrun interrupt
    ltdc.ier.modify(|_, w| {
        w.terrie().bit(true); // TRANSFER_ERROR_INTERRUPT_ENABLE
        w.fuie().bit(true); // FIFO_UNDERRUN_INTERRUPT_ENABLE
        w
    });

    // enable LTDC
    ltdc.gcr.modify(|_, w| w.ltdcen().bit(true));

    // configure layers

    // configure horizontal start and stop position
    ltdc.l1whpcr.modify(|_, w| unsafe {
        w.whstpos().bits(0 + 41 + 13); // window_horizontal_start_position
        w.whsppos().bits(WIDTH + 41 + 13 - 1); // window_horizontal_stop_position
        w
    });
    ltdc.l2whpcr.modify(|_, w| unsafe {
        w.whstpos().bits(0 + 41 + 13); // window_horizontal_start_position
        w.whsppos().bits(WIDTH + 41 + 13 - 1); // window_horizontal_stop_position
        w
    });

    // configure vertical start and stop position
    ltdc.l1wvpcr.modify(|_, w| unsafe {
        w.wvstpos().bits(0 + 10 + 2); // window_vertical_start_position
        w.wvsppos().bits(HEIGHT + 10 + 2 - 1); // window_vertical_stop_position
        w
    });
    ltdc.l2wvpcr.modify(|_, w| unsafe {
        w.wvstpos().bits(0 + 10 + 2); // window_vertical_start_position
        w.wvsppos().bits(HEIGHT + 10 + 2 - 1); // window_vertical_stop_position
        w
    });

    // specify pixed format
    ltdc.l1pfcr.modify(|_, w| unsafe { w.pf().bits(0b000) }); // set_pixel_format to ARGB8888
    ltdc.l2pfcr.modify(|_, w| unsafe { w.pf().bits(0b111) }); // set_pixel_format to AL88

    // configure default color values
    ltdc.l1dccr.modify(|_, w| unsafe {
        w.dcalpha().bits(0);
        w.dcred().bits(0);
        w.dcgreen().bits(0);
        w.dcblue().bits(0);
        w
    });
    ltdc.l2dccr.modify(|_, w| unsafe {
        w.dcalpha().bits(0);
        w.dcred().bits(0);
        w.dcgreen().bits(0);
        w.dcblue().bits(0);
        w
    });

    // specify constant alpha value
    ltdc.l1cacr.modify(|_, w| unsafe { w.consta().bits(255) }); // constant_alpha
    ltdc.l2cacr.modify(|_, w| unsafe { w.consta().bits(255) }); // constant_alpha

    // specify blending factors
    ltdc.l1bfcr.modify(|_, w| unsafe {
        w.bf1().bits(0b110); // set_blending_factor_1 to PixelAlphaTimesConstantAlpha
        w.bf2().bits(0b111); // set_blending_factor_2 to OneMinusPixelAlphaTimesConstantAlpha
        w
    });
    ltdc.l2bfcr.modify(|_, w| unsafe {
        w.bf1().bits(0b110); // set_blending_factor_1 to PixelAlphaTimesConstantAlpha
        w.bf2().bits(0b111); // set_blending_factor_2 to OneMinusPixelAlphaTimesConstantAlpha
        w
    });

    // configure color frame buffer start address
    ltdc.l1cfbar
        .modify(|_, w| unsafe { w.cfbadd().bits(LAYER_1_START as u32) });
    ltdc.l2cfbar
        .modify(|_, w| unsafe { w.cfbadd().bits(LAYER_2_START as u32) });

    // configure color frame buffer line length and pitch
    ltdc.l1cfblr.modify(|_, w| unsafe {
        w.cfbp().bits(WIDTH * LAYER_1_OCTETS_PER_PIXEL); // pitch
        w.cfbll().bits(WIDTH * LAYER_1_OCTETS_PER_PIXEL + 3); // line_length
        w
    });
    ltdc.l2cfblr.modify(|_, w| unsafe {
        w.cfbp().bits(WIDTH * LAYER_2_OCTETS_PER_PIXEL); // pitch
        w.cfbll().bits(WIDTH * LAYER_2_OCTETS_PER_PIXEL + 3); // line_length
        w
    });

    // configure frame buffer line number
    ltdc.l1cfblnr
        .modify(|_, w| unsafe { w.cfblnbr().bits(HEIGHT) }); // line_number
    ltdc.l2cfblnr
        .modify(|_, w| unsafe { w.cfblnbr().bits(HEIGHT) }); // line_number

    // enable layers
    ltdc.l1cr.modify(|_, w| w.len().set_bit());
    ltdc.l2cr.modify(|_, w| w.len().set_bit());

    // reload shadow registers
    ltdc.srcr.modify(|_, w| w.imr().set_bit()); // IMMEDIATE_RELOAD

    // init DMA2D graphic
}
