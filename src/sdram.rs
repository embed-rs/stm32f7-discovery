use board::rcc::Rcc;
use board::fmc::Fmc;
use system_clock;
use embedded::interfaces::gpio::Gpio;

pub fn init(rcc: &mut Rcc, fmc: &mut Fmc, gpio: &mut Gpio) {
    config_pins(gpio);

    // Enable FMC clock
    rcc.ahb3enr.update(|r| r.set_fmcen(true));

    // Reset FMC module
    rcc.ahb3rstr.update(|r| r.set_fmcrst(true));
    rcc.ahb3rstr.update(|r| r.set_fmcrst(false));

    // SDRAM contol register
    fmc.sdcr1
        .update(|r| {
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
    fmc.sdtr1
        .update(|r| {
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
    fmc.sdrtr
        .update(|r| {
                    r.set_count(0x301);
                    r.set_reie(false);
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

fn config_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::{OutputType, OutputSpeed, AlternateFunction, Resistor};
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;

    let sdclk = (PortG, Pin8);
    let sdcke0 = (PortC, Pin3);
    let sdcke1 = (PortB, Pin5);
    let sdne0 = (PortH, Pin3);
    let sdne1 = (PortH, Pin6);
    let a0 = (PortF, Pin0);
    let a1 = (PortF, Pin1);
    let a2 = (PortF, Pin2);
    let a3 = (PortF, Pin3);
    let a4 = (PortF, Pin4);
    let a5 = (PortF, Pin5);
    let a6 = (PortF, Pin12);
    let a7 = (PortF, Pin13);
    let a8 = (PortF, Pin14);
    let a9 = (PortF, Pin15);
    let a10 = (PortG, Pin0);
    let a11 = (PortG, Pin1);
    let a12 = (PortG, Pin2);
    let d0 = (PortD, Pin14);
    let d1 = (PortD, Pin15);
    let d2 = (PortD, Pin0);
    let d3 = (PortD, Pin1);
    let d4 = (PortE, Pin7);
    let d5 = (PortE, Pin8);
    let d6 = (PortE, Pin9);
    let d7 = (PortE, Pin10);
    let d8 = (PortE, Pin11);
    let d9 = (PortE, Pin12);
    let d10 = (PortE, Pin13);
    let d11 = (PortE, Pin14);
    let d12 = (PortE, Pin15);
    let d13 = (PortD, Pin8);
    let d14 = (PortD, Pin9);
    let d15 = (PortD, Pin10);
    let ba0 = (PortG, Pin4);
    let ba1 = (PortG, Pin5);
    let nras = (PortF, Pin11);
    let ncas = (PortG, Pin15);
    let sdnwe = (PortH, Pin5);

    let pins = [sdclk, sdcke0, sdcke1, sdne0, sdne1, a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10,
                a11, a12, d0, d1, d2, d3, d4, d5, d6, d7, d8, d9, d10, d11, d12, d13, d14, d15,
                ba0, ba1, nras, ncas, sdnwe];
    gpio.to_alternate_function_all(&pins,
                                   AlternateFunction::AF12,
                                   OutputType::PushPull,
                                   OutputSpeed::High,
                                   Resistor::PullUp)
        .unwrap();
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

    fmc.sdcmr
        .update(|cmr| {
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
