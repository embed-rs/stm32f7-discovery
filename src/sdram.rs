use svd_board::rcc::Rcc;
use svd_board::fmc::Fmc;
use system_clock;
use gpio::{self, GpioController};

pub fn init(rcc: &mut Rcc, fmc: &mut Fmc, gpio: &mut GpioController) {
    config_pins(gpio);

    // Enable FMC clock
    rcc.ahb3enr.update(|r| r.set_fmcen(true));

    // Reset FMC module
    rcc.ahb3rstr.update(|r| r.set_fmcrst(true));
    rcc.ahb3rstr.update(|r| r.set_fmcrst(false));

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

fn config_pins(gpio: &mut GpioController) {
    let t = gpio::Type::PushPull;
    let s = gpio::Speed::High;
    let a = gpio::AlternateFunction::AF12;
    let r = gpio::Resistor::PullUp;

    let sdclk = gpio.pins.g.8.take().unwrap();
    let sdcke0 = gpio.pins.c.3.take().unwrap();
    let sdcke1 = gpio.pins.h.7.take().unwrap();
    let sdne0 = gpio.pins.h.3.take().unwrap();
    let sdne1 = gpio.pins.h.6.take().unwrap();
    let a0 = gpio.pins.f.0.take().unwrap();
    let a1 = gpio.pins.f.1.take().unwrap();
    let a2 = gpio.pins.f.2.take().unwrap();
    let a3 = gpio.pins.f.3.take().unwrap();
    let a4 = gpio.pins.f.4.take().unwrap();
    let a5 = gpio.pins.f.5.take().unwrap();
    let a6 = gpio.pins.f.12.take().unwrap();
    let a7 = gpio.pins.f.13.take().unwrap();
    let a8 = gpio.pins.f.14.take().unwrap();
    let a9 = gpio.pins.f.15.take().unwrap();
    let a10 = gpio.pins.g.0.take().unwrap();
    let a11 = gpio.pins.g.1.take().unwrap();
    let a12 = gpio.pins.g.2.take().unwrap();
    let d0 = gpio.pins.d.14.take().unwrap();
    let d1 = gpio.pins.d.15.take().unwrap();
    let d2 = gpio.pins.d.0.take().unwrap();
    let d3 = gpio.pins.d.1.take().unwrap();
    let d4 = gpio.pins.e.7.take().unwrap();
    let d5 = gpio.pins.e.8.take().unwrap();
    let d6 = gpio.pins.e.9.take().unwrap();
    let d7 = gpio.pins.e.10.take().unwrap();
    let d8 = gpio.pins.e.11.take().unwrap();
    let d9 = gpio.pins.e.12.take().unwrap();
    let d10 = gpio.pins.e.13.take().unwrap();
    let d11 = gpio.pins.e.14.take().unwrap();
    let d12 = gpio.pins.e.15.take().unwrap();
    let d13 = gpio.pins.d.8.take().unwrap();
    let d14 = gpio.pins.d.9.take().unwrap();
    let d15 = gpio.pins.d.10.take().unwrap();
    let ba0 = gpio.pins.g.4.take().unwrap();
    let ba1 = gpio.pins.g.5.take().unwrap();
    let nras = gpio.pins.f.11.take().unwrap();
    let ncas = gpio.pins.g.15.take().unwrap();
    let sdnwe = gpio.pins.h.5.take().unwrap();
    let nbl0 = gpio.pins.e.0.take().unwrap();
    let nbl1 = gpio.pins.e.1.take().unwrap();
    let nbl2 = gpio.pins.i.4.take().unwrap();
    let nbl3 = gpio.pins.i.5.take().unwrap();

    gpio.to_alternate_function(sdclk, t, s, a, r);
    gpio.to_alternate_function(sdcke0, t, s, a, r);
    gpio.to_alternate_function(sdcke1, t, s, a, r);
    gpio.to_alternate_function(sdne0, t, s, a, r);
    gpio.to_alternate_function(sdne1, t, s, a, r);
    gpio.to_alternate_function(a0, t, s, a, r);
    gpio.to_alternate_function(a1, t, s, a, r);
    gpio.to_alternate_function(a2, t, s, a, r);
    gpio.to_alternate_function(a3, t, s, a, r);
    gpio.to_alternate_function(a4, t, s, a, r);
    gpio.to_alternate_function(a5, t, s, a, r);
    gpio.to_alternate_function(a6, t, s, a, r);
    gpio.to_alternate_function(a7, t, s, a, r);
    gpio.to_alternate_function(a8, t, s, a, r);
    gpio.to_alternate_function(a9, t, s, a, r);
    gpio.to_alternate_function(a10, t, s, a, r);
    gpio.to_alternate_function(a11, t, s, a, r);
    gpio.to_alternate_function(a12, t, s, a, r);
    gpio.to_alternate_function(d0, t, s, a, r);
    gpio.to_alternate_function(d1, t, s, a, r);
    gpio.to_alternate_function(d2, t, s, a, r);
    gpio.to_alternate_function(d3, t, s, a, r);
    gpio.to_alternate_function(d4, t, s, a, r);
    gpio.to_alternate_function(d5, t, s, a, r);
    gpio.to_alternate_function(d6, t, s, a, r);
    gpio.to_alternate_function(d7, t, s, a, r);
    gpio.to_alternate_function(d8, t, s, a, r);
    gpio.to_alternate_function(d9, t, s, a, r);
    gpio.to_alternate_function(d10, t, s, a, r);
    gpio.to_alternate_function(d11, t, s, a, r);
    gpio.to_alternate_function(d12, t, s, a, r);
    gpio.to_alternate_function(d13, t, s, a, r);
    gpio.to_alternate_function(d14, t, s, a, r);
    gpio.to_alternate_function(d15, t, s, a, r);
    gpio.to_alternate_function(ba0, t, s, a, r);
    gpio.to_alternate_function(ba1, t, s, a, r);
    gpio.to_alternate_function(nras, t, s, a, r);
    gpio.to_alternate_function(ncas, t, s, a, r);
    gpio.to_alternate_function(sdnwe, t, s, a, r);
    gpio.to_alternate_function(nbl0, t, s, a, r);
    gpio.to_alternate_function(nbl1, t, s, a, r);
    gpio.to_alternate_function(nbl2, t, s, a, r);
    gpio.to_alternate_function(nbl3, t, s, a, r);
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
