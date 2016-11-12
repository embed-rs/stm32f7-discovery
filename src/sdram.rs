use svd_board::rcc::Rcc;
use svd_board::fmc::Fmc;
use svd_board::gpioc::Gpioc;
use svd_board::gpiod::Gpiod;
use svd_board::gpioe::Gpioe;
use svd_board::gpiof::Gpiof;
use svd_board::gpiog::Gpiog;
use svd_board::gpioh::Gpioh;
use svd_board::gpioi::Gpioi;
use system_clock;

pub fn init(rcc: &mut Rcc,
            fmc: &mut Fmc,
            gpio_c: &mut Gpioc,
            gpio_d: &mut Gpiod,
            gpio_e: &mut Gpioe,
            gpio_f: &mut Gpiof,
            gpio_g: &mut Gpiog,
            gpio_h: &mut Gpioh,
            gpio_i: &mut Gpioi) {

    config_pins(gpio_c, gpio_d, gpio_e, gpio_f, gpio_g, gpio_h, gpio_i);

    // Enable FMC clock
    rcc.ahb3enr.update(|r| r.set_fmcen(true));

    // Reset FMC module
    rcc.ahb3rstr.update(|r| r.set_fmcrst(true));
    rcc.ahb3rstr.update(|r| r.set_fmcrst(false));

    // init2();

    // SDRAM contol register
    fmc.sdcr1.update(|r| {
        r.set_nc(8 - 8); // number_of_column_address_bits
        r.set_nr(12 - 11); // number_of_row_address_bits
        r.set_mwid(0b01 /* = 16 */); // data_bus_width
        r.set_nb(true /* = 4 */); // number_of_internal_banks
        r.set_cas(2); // cas_latency
        r.set_wp(false); // write_protection
        r.set_rburst(false); // burst_read
        r.set_sdclk(2); // enable_sdram_clock
    });

    // SDRAM timings
    fmc.sdtr1.update(|r| {
        r.set_tmrd(2 - 1); // load_mode_register_to_active
        r.set_txsr(7 - 1); // exit_self_refresh_delay
        r.set_tras(4 - 1); // self_refresh_time
        r.set_trc(7 - 1); // row_cycle_delay
        r.set_twr(2 - 1); // recovery_delay
        r.set_trp(2 - 1); // row_precharge_delay
        r.set_trcd(2 - 1); // row_to_column_delay
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
    fmc.sdrtr.update(|r| {
        r.set_count(0x301);
        r.set_reie(false);
    });

    // test sdram
    use core::ptr;

    let ptr1 = 0xC000_0000 as *mut u32;
    let ptr2 = 0xC053_6170 as *mut u32;
    let ptr3 = 0xC07f_fffc as *mut u32;

    unsafe {
        ptr::write_volatile(ptr1, 0xcafebabe);
        ptr::write_volatile(ptr2, 0xdeadbeaf);
        ptr::write_volatile(ptr3, 0x0deafbee);
        assert_eq!(ptr::read_volatile(ptr1), 0xcafebabe);
        assert_eq!(ptr::read_volatile(ptr2), 0xdeadbeaf);
        assert_eq!(ptr::read_volatile(ptr3), 0x0deafbee);
    }
}

fn config_pins(gpio_c: &mut Gpioc,
               gpio_d: &mut Gpiod,
               gpio_e: &mut Gpioe,
               gpio_f: &mut Gpiof,
               gpio_g: &mut Gpiog,
               gpio_h: &mut Gpioh,
               gpio_i: &mut Gpioi) {

    // configure sdram pins
    const ALTERNATE_FN: u8 = 0b10;
    const ALTERNATE_FN_NUMBER: u8 = 12;
    const SPEED_HIGH: u8 = 0b10;

    gpio_c.moder.update(|r| {
        r.set_moder3(ALTERNATE_FN); // SDRAM Bank 1 Clock Enable pin (SDCKE0)
    });
    gpio_c.ospeedr.update(|r| {
        r.set_ospeedr3(SPEED_HIGH); // SDRAM Bank 1 Clock Enable pin (SDCKE0)
    });
    gpio_c.afrl.update(|r| {
        r.set_afrl3(ALTERNATE_FN_NUMBER); // SDRAM Bank 1 Clock Enable pin (SDCKE0)
    });

    gpio_d.moder.update(|r| {
        r.set_moder0(ALTERNATE_FN); // data pin D2
        r.set_moder1(ALTERNATE_FN); // data pin D3
        r.set_moder8(ALTERNATE_FN); // data pin D13
        r.set_moder9(ALTERNATE_FN); // data pin D14
        r.set_moder10(ALTERNATE_FN); // data pin D15
        r.set_moder14(ALTERNATE_FN); // data pin D0
        r.set_moder15(ALTERNATE_FN); // data pin D1
    });
    gpio_d.ospeedr.update(|r| {
        r.set_ospeedr0(SPEED_HIGH); // data pin D2
        r.set_ospeedr1(SPEED_HIGH); // data pin D3
        r.set_ospeedr8(SPEED_HIGH); // data pin D13
        r.set_ospeedr9(SPEED_HIGH); // data pin D14
        r.set_ospeedr10(SPEED_HIGH); // data pin D15
        r.set_ospeedr14(SPEED_HIGH); // data pin D0
        r.set_ospeedr15(SPEED_HIGH); // data pin D1
    });
    gpio_d.afrl.update(|r| {
        r.set_afrl0(ALTERNATE_FN_NUMBER); // data pin D2
        r.set_afrl1(ALTERNATE_FN_NUMBER); // data pin D3
    });
    gpio_d.afrh.update(|r| {
        r.set_afrh8(ALTERNATE_FN_NUMBER); // data pin D13
        r.set_afrh9(ALTERNATE_FN_NUMBER); // data pin D14
        r.set_afrh10(ALTERNATE_FN_NUMBER); // data pin D15
        r.set_afrh14(ALTERNATE_FN_NUMBER); // data pin D0
        r.set_afrh15(ALTERNATE_FN_NUMBER); // data pin D1
    });

    gpio_e.moder.update(|r| {
        r.set_moder0(ALTERNATE_FN); // NBL0
        r.set_moder1(ALTERNATE_FN); // NBL1
        r.set_moder7(ALTERNATE_FN); // data pin D4
        r.set_moder8(ALTERNATE_FN); // data pin D5
        r.set_moder9(ALTERNATE_FN); // data pin D6
        r.set_moder10(ALTERNATE_FN); // data pin D7
        r.set_moder11(ALTERNATE_FN); // data pin D8
        r.set_moder12(ALTERNATE_FN); // data pin D9
        r.set_moder13(ALTERNATE_FN); // data pin D10
        r.set_moder14(ALTERNATE_FN); // data pin D11
        r.set_moder15(ALTERNATE_FN); // data pin D12
    });
    gpio_e.ospeedr.update(|r| {
        r.set_ospeedr0(SPEED_HIGH); // NBL0
        r.set_ospeedr1(SPEED_HIGH); // NBL1
        r.set_ospeedr7(SPEED_HIGH); // data pin D4
        r.set_ospeedr8(SPEED_HIGH); // data pin D5
        r.set_ospeedr9(SPEED_HIGH); // data pin D6
        r.set_ospeedr10(SPEED_HIGH); // data pin D7
        r.set_ospeedr11(SPEED_HIGH); // data pin D8
        r.set_ospeedr12(SPEED_HIGH); // data pin D9
        r.set_ospeedr13(SPEED_HIGH); // data pin D10
        r.set_ospeedr14(SPEED_HIGH); // data pin D11
        r.set_ospeedr15(SPEED_HIGH); // data pin D12
    });
    gpio_e.afrl.update(|r| {
        r.set_afrl0(ALTERNATE_FN_NUMBER); // NBL0
        r.set_afrl1(ALTERNATE_FN_NUMBER); // NBL1
        r.set_afrl7(ALTERNATE_FN_NUMBER); // data pin D4
    });
    gpio_e.afrh.update(|r| {
        r.set_afrh8(ALTERNATE_FN_NUMBER); // data pin D5
        r.set_afrh9(ALTERNATE_FN_NUMBER); // data pin D6
        r.set_afrh10(ALTERNATE_FN_NUMBER); // data pin D7
        r.set_afrh11(ALTERNATE_FN_NUMBER); // data pin D8
        r.set_afrh12(ALTERNATE_FN_NUMBER); // data pin D9
        r.set_afrh13(ALTERNATE_FN_NUMBER); // data pin D10
        r.set_afrh14(ALTERNATE_FN_NUMBER); // data pin D11
        r.set_afrh15(ALTERNATE_FN_NUMBER); // data pin D12
    });

    gpio_f.moder.update(|r| {
        r.set_moder0(ALTERNATE_FN); // address pin A0
        r.set_moder1(ALTERNATE_FN); // address pin A1
        r.set_moder2(ALTERNATE_FN); // address pin A2
        r.set_moder3(ALTERNATE_FN); // address pin A3
        r.set_moder4(ALTERNATE_FN); // address pin A4
        r.set_moder5(ALTERNATE_FN); // address pin A5
        r.set_moder11(ALTERNATE_FN); // row address strobe pin (NRAS)
        r.set_moder12(ALTERNATE_FN); // address pin A6
        r.set_moder13(ALTERNATE_FN); // address pin A7
        r.set_moder14(ALTERNATE_FN); // address pin A8
        r.set_moder15(ALTERNATE_FN); // address pin A9
    });
    gpio_f.ospeedr.update(|r| {
        r.set_ospeedr0(SPEED_HIGH); // address pin A0
        r.set_ospeedr1(SPEED_HIGH); // address pin A1
        r.set_ospeedr2(SPEED_HIGH); // address pin A2
        r.set_ospeedr3(SPEED_HIGH); // address pin A3
        r.set_ospeedr4(SPEED_HIGH); // address pin A4
        r.set_ospeedr5(SPEED_HIGH); // address pin A5
        r.set_ospeedr11(SPEED_HIGH); // row address strobe pin (NRAS)
        r.set_ospeedr12(SPEED_HIGH); // address pin A6
        r.set_ospeedr13(SPEED_HIGH); // address pin A7
        r.set_ospeedr14(SPEED_HIGH); // address pin A8
        r.set_ospeedr15(SPEED_HIGH); // address pin A9
    });
    gpio_f.afrl.update(|r| {
        r.set_afrl0(ALTERNATE_FN_NUMBER); // address pin A0
        r.set_afrl1(ALTERNATE_FN_NUMBER); // address pin A1
        r.set_afrl2(ALTERNATE_FN_NUMBER); // address pin A2
        r.set_afrl3(ALTERNATE_FN_NUMBER); // address pin A3
        r.set_afrl4(ALTERNATE_FN_NUMBER); // address pin A4
        r.set_afrl5(ALTERNATE_FN_NUMBER); // address pin A5
    });
    gpio_f.afrh.update(|r| {
        r.set_afrh11(ALTERNATE_FN_NUMBER); // row address strobe pin (NRAS)
        r.set_afrh12(ALTERNATE_FN_NUMBER); // address pin A6
        r.set_afrh13(ALTERNATE_FN_NUMBER); // address pin A7
        r.set_afrh14(ALTERNATE_FN_NUMBER); // address pin A8
        r.set_afrh15(ALTERNATE_FN_NUMBER); // address pin A9
    });

    gpio_g.moder.update(|r| {
        r.set_moder0(ALTERNATE_FN); // address pin 10 (A10)
        r.set_moder1(ALTERNATE_FN); // address pin 11 (A11)
        r.set_moder2(ALTERNATE_FN); // address pin 12 (A12)
        r.set_moder4(ALTERNATE_FN); // bank address pin 0 (BA0)
        r.set_moder5(ALTERNATE_FN); // bank address pin 1 (BA1)
        r.set_moder8(ALTERNATE_FN); // SDRAM clock pin (SDCLK)
        r.set_moder15(ALTERNATE_FN); // row address strobe pin (NCAS)
    });
    gpio_g.ospeedr.update(|r| {
        r.set_ospeedr0(SPEED_HIGH); // address pin 10 (A10)
        r.set_ospeedr1(SPEED_HIGH); // address pin 11 (A11)
        r.set_ospeedr2(SPEED_HIGH); // address pin 12 (A12)
        r.set_ospeedr4(SPEED_HIGH); // bank address pin 0 (BA0)
        r.set_ospeedr5(SPEED_HIGH); // bank address pin 1 (BA1)
        r.set_ospeedr8(SPEED_HIGH); // SDRAM clock pin (SDCLK)
        r.set_ospeedr15(SPEED_HIGH); // row address strobe pin (NCAS)
    });
    gpio_g.afrl.update(|r| {
        r.set_afrl0(ALTERNATE_FN_NUMBER); // address pin 10 (A10)
        r.set_afrl1(ALTERNATE_FN_NUMBER); // address pin 11 (A11)
        r.set_afrl2(ALTERNATE_FN_NUMBER); // address pin 12 (A12)
        r.set_afrl4(ALTERNATE_FN_NUMBER); // bank address pin 0 (BA0)
        r.set_afrl5(ALTERNATE_FN_NUMBER); // bank address pin 1 (BA1)
    });
    gpio_g.afrh.update(|r| {
        r.set_afrh8(ALTERNATE_FN_NUMBER); // SDRAM clock pin (SDCLK)
        r.set_afrh15(ALTERNATE_FN_NUMBER); // row address strobe pin (NCAS)
    });

    gpio_h.moder.update(|r| {
        r.set_moder3(ALTERNATE_FN); // SDRAM Bank 1 Chip Enable (SDNE0)
        r.set_moder5(ALTERNATE_FN); // write enable pin (SDNWE)
        r.set_moder6(ALTERNATE_FN); // SDRAM Bank 2 Chip Enable (SDNE1)
        r.set_moder7(ALTERNATE_FN); // SDRAM Bank 2 Clock Enable pin (SDCKE1)
    });
    gpio_h.ospeedr.update(|r| {
        r.set_ospeedr3(SPEED_HIGH); // SDRAM Bank 1 Chip Enable (SDNE0)
        r.set_ospeedr5(SPEED_HIGH); // write enable pin (SDNWE)
        r.set_ospeedr6(SPEED_HIGH); // SDRAM Bank 2 Chip Enable (SDNE1)
        r.set_ospeedr7(SPEED_HIGH); // SDRAM Bank 2 Clock Enable pin (SDCKE1)
    });
    gpio_h.afrl.update(|r| {
        r.set_afrl3(ALTERNATE_FN_NUMBER); // SDRAM Bank 1 Chip Enable (SDNE0)
        r.set_afrl5(ALTERNATE_FN_NUMBER); // write enable pin (SDNWE)
        r.set_afrl6(ALTERNATE_FN_NUMBER); // SDRAM Bank 2 Chip Enable (SDNE1)
        r.set_afrl7(ALTERNATE_FN_NUMBER); // SDRAM Bank 2 Clock Enable pin (SDCKE1)
    });

    gpio_i.moder.update(|r| {
        // output Byte Mask for write accesses pins
        r.set_moder4(ALTERNATE_FN); // NBL2
        r.set_moder5(ALTERNATE_FN); // NBL3
    });
    gpio_i.ospeedr.update(|r| {
        r.set_ospeedr4(SPEED_HIGH); // NBL2
        r.set_ospeedr5(SPEED_HIGH); // NBL3
    });
    gpio_i.afrl.update(|r| {
        r.set_afrl4(ALTERNATE_FN_NUMBER); // NBL2
        r.set_afrl5(ALTERNATE_FN_NUMBER); // NBL3
    });
}

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

fn send_fmc_command(fmc: &mut Fmc, bank: Bank, command: Command, auto_refresh: u8, modereg: u16) {
    assert!(!fmc.sdsr.read().busy());

    fmc.sdcmr.update(|cmr| {
        match bank {
            Bank::One => cmr.set_ctb1(true),
            Bank::Two => cmr.set_ctb2(true),
            Bank::Both => {
                cmr.set_ctb1(true);
                cmr.set_ctb2(true);
            }
        }

        cmr.set_mode(command as u8);
        cmr.set_nrfs(auto_refresh); // number_of_auto_refresh
        cmr.set_mrd(modereg); // mode_register_definition
    });

    while fmc.sdsr.read().busy() {
        // wait
    }
}
