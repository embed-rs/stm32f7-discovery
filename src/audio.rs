use board::rcc::Rcc;
use board::sai::{self, Sai};
use embedded::interfaces::gpio::Gpio;
use i2c;
use system_clock;

const WM8994_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0011010);

pub fn init_wm8994(i2c_3: &mut i2c::I2C) -> Result<(), i2c::Error> {
    i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {
        // read and check device family ID
        assert_eq!(conn.read(0).ok(), Some(0x8994));
        // reset device
        try!(conn.write(0, 0));

        // wm8994 Errata Work-Arounds
        try!(conn.write(0x102, 0x0003));
        try!(conn.write(0x817, 0x0000));
        try!(conn.write(0x102, 0x0000));

        // Enable VMID soft start (fast), Start-up Bias Current Enabled
        try!(conn.write(0x39, 0x006C));

        // Enable bias generator, Enable VMID
        try!(conn.write(0x01, 0x0003));

        system_clock::wait(50);

        // INPUT_DEVICE_DIGITAL_MICROPHONE_2 :

        // Enable AIF1ADC2 (Left), Enable AIF1ADC2 (Right)
        // Enable DMICDAT2 (Left), Enable DMICDAT2 (Right)
        // Enable Left ADC, Enable Right ADC
        try!(conn.write(0x04, 0x0C30));

        // Enable AIF1 DRC2 Signal Detect & DRC in AIF1ADC2 Left/Right Timeslot 1
        try!(conn.write(0x450, 0x00DB));

        // Disable IN1L, IN1R, IN2L, IN2R, Enable Thermal sensor & shutdown
        try!(conn.write(0x02, 0x6000));

        // Enable the DMIC2(Left) to AIF1 Timeslot 1 (Left) mixer path
        try!(conn.write(0x608, 0x0002));

        // Enable the DMIC2(Right) to AIF1 Timeslot 1 (Right) mixer path
        try!(conn.write(0x609, 0x0002));

        // GPIO1 pin configuration GP1_DIR = output, GP1_FN = AIF1 DRC2 signal detect
        try!(conn.write(0x700, 0x000E));

        // Clock Configurations

        // AIF1 Sample Rate = 16 (KHz), ratio=256
        try!(conn.write(0x210, 0x0033));

        // AIF1 Word Length = 16-bits, AIF1 Format = I2S (Default Register Value)
        try!(conn.write(0x300, 0x4010));

        // slave mode
        try!(conn.write(0x302, 0x0000));

        // Enable the DSP processing clock for AIF1, Enable the core clock
        try!(conn.write(0x208, 0x000A));

        // Enable AIF1 Clock, AIF1 Clock Source = MCLK1 pin
        try!(conn.write(0x200, 0x0001));

        // Enable Microphone bias 1 generator, Enable VMID
        try!(conn.write(0x01, 0x0013));

        // ADC oversample enable
        try!(conn.write(0x620, 0x0002));

        // AIF ADC2 HPF enable, HPF cut = voice mode 1 fc=127Hz at fs=8kHz
        try!(conn.write(0x411, 0x3800));

        // set volume

        let convertedvol = 239; // 100(+17.625dB)

        // Left AIF1 ADC1 volume
        try!(conn.write(0x400, convertedvol | 0x100));

        // Right AIF1 ADC1 volume
        try!(conn.write(0x401, convertedvol | 0x100));

        // Left AIF1 ADC2 volume
        try!(conn.write(0x404, convertedvol | 0x100));

        // Right AIF1 ADC2 volume
        try!(conn.write(0x405, convertedvol | 0x100));

        Ok(())
    })
}

pub fn init_sai_2(sai: &mut Sai, rcc: &mut Rcc) {
    let audio_frequency = 16000;

    // disable block a and block b
    sai.acr1.update(|r| r.set_saiaen(false)); // audio_block_enable
    sai.bcr1.update(|r| r.set_saiben(false)); // audio_block_enable
    while sai.acr1.read().saiaen() {}
    while sai.bcr1.read().saiben() {}

    // enable sai2 clock
    rcc.apb2enr.update(|r| r.set_sai2en(true));

    // Disabled All interrupt and clear all the flag
    sai.bim.write(Default::default());
    let mut clear_all_flags = sai::Bclrfr::default();
    clear_all_flags.set_lfsdet(true); // Clear late frame synchronization detection flag
    clear_all_flags.set_cafsdet(true); // Clear anticipated frame synchronization detection flag
    clear_all_flags.set_cnrdy(true); // Clear codec not ready flag
    clear_all_flags.set_wckcfg(true); // Clear wrong clock configuration flag
    clear_all_flags.set_mutedet(true); // Clear mute detection flag
    clear_all_flags.set_ovrudr(true); // Clear overrun / underrun
    sai.bclrfr.write(clear_all_flags);

    // Flush the fifo
    sai.bcr2.update(|r| r.set_fflus(true)); // fifo_flush


    // PLL clock is set depending on the AudioFreq (44.1khz vs 48khz groups)

    // I2S clock config
    // PLLI2S_VCO: VCO_344M
    // I2S_CLK(first level) = PLLI2S_VCO/PLLI2SQ = 344/7 = 49.142 Mhz
    // I2S_CLK_x = I2S_CLK(first level)/PLLI2SDIVQ = 49.142/1 = 49.142 Mhz

    // Configure SAI2 Clock source
    rcc.dkcfgr1.update(|r| r.set_sai2sel(0)); // sai2_clock_source plli2s

    // Disable the PLLI2S
    rcc.cr.update(|r| r.set_plli2son(false));
    while rcc.cr.read().plli2srdy() {}

    // Configure the PLLI2S division factors
    // PLLI2S_VCO Input  = PLL_SOURCE/PLLM
    // PLLI2S_VCO Output = PLLI2S_VCO Input * PLLI2SN
    // SAI_CLK(first level) = PLLI2S_VCO Output/PLLI2SQ
    rcc.plli2scfgr.update(|r| {
                              r.set_plli2sn(344);
                              r.set_plli2sq(7);
                          });

    // SAI_CLK_x = SAI_CLK(first level)/PLLI2SDIVQ
    rcc.dkcfgr1.update(|r| r.set_plli2sdiv(1 - 1));

    // Enable the PLLI2S
    rcc.cr.update(|r| r.set_plli2son(true));
    while !rcc.cr.read().plli2srdy() {}


    // configure sai registers

    // disable synchronization outputs
    sai.gcr.update(|r| r.set_syncout(0)); // NoSyncOutput

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
            let vcoinput = 25000000 / u32::from(rcc.pllcfgr.read().pllm());

            // PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAIN
            // SAI_CLK(first level) = PLLSAI_VCO Output/PLLSAIQ
            let tmpreg = u32::from(rcc.pllsaicfgr.read().pllsaiq());
            let frequency = (vcoinput * u32::from(rcc.pllsaicfgr.read().pllsain())) / tmpreg;

            // SAI_CLK_x = SAI_CLK(first level)/PLLSAIDIVQ
            let tmpreg = u32::from(rcc.dkcfgr1.read().pllsaidivq()) + 1;
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

    let mut acr1 = sai::Acr1::default();
    acr1.set_mode(0b01); // MasterReceiver
    acr1.set_prtcfg(0b00); // protocol free
    acr1.set_ds(0b100); // data_size 16 bits
    acr1.set_lsbfirst(false);
    acr1.set_ckstr(true); // clock_strobing_edge
    acr1.set_syncen(0b00); // synchronization asynchronous
    acr1.set_mono(false);
    acr1.set_out_dri(true); // output_drive
    acr1.set_nodiv(false); // no_divider
    acr1.set_mcjdiv(mckdiv as u8); // master_clock_divider8
    sai.acr1.write(acr1);

    // configure cr2
    let mut acr2 = sai::Acr2::default();
    acr2.set_fth(0b001); // fifo_threshold QuarterFifo
    acr2.set_tris(false); // tristate_management
    acr2.set_comp(0b00); // companding_mode None
    sai.acr2.write(acr2);

    // configure frame
    let mut afrcr = sai::Afrcr::default();
    afrcr.set_frl(64 - 1); // frame_length
    afrcr.set_fsall(32 - 1); // sync_active_level_length
    afrcr.set_fsdef(true); // frame_sync_definition
    afrcr.set_fspol(false); // frame_sync_polarity
    afrcr.set_fsoff(true); // frame_sync_offset
    sai.afrcr.write(afrcr);

    // configure slot
    let mut aslotr = sai::Aslotr::default();
    aslotr.set_fboff(0); // first_bit_offset
    aslotr.set_slotsz(0b00); // slot_size DataSize
    aslotr.set_nbslot(4 - 1); // number_of_slots
    aslotr.set_sloten(1 << 1 | 1 << 3); // enable_slots
    sai.aslotr.write(aslotr);

    // Initialize SAI2 block B in SLAVE RX synchronous from SAI2 block A

    // configure cr1
    let mut bcr1 = sai::Bcr1::default();
    bcr1.set_mode(0b11); // SlaveReceiver
    bcr1.set_prtcfg(0b00); // protocol free
    bcr1.set_ds(0b100); // data_size 16 bits
    bcr1.set_lsbfirst(false);
    bcr1.set_ckstr(true); // clock_strobing_edge
    bcr1.set_syncen(0b01); // synchronization SynchronousWithOtherSubBlock
    bcr1.set_mono(false);
    bcr1.set_out_dri(true); // output_drive
    bcr1.set_nodiv(false); // no_divider
    bcr1.set_mcjdiv(mckdiv as u8); // master_clock_divider8
    sai.bcr1.write(bcr1);

    // configure cr2
    let mut bcr2 = sai::Bcr2::default();
    bcr2.set_fth(0b001); // fifo_threshold QuarterFifo
    bcr2.set_tris(false); // tristate_management
    bcr2.set_comp(0b00); // companding_mode None
    sai.bcr2.write(bcr2);

    // configure frame
    let mut bfrcr = sai::Bfrcr::default();
    bfrcr.set_frl(64 - 1); // frame_length
    bfrcr.set_fsall(32 - 1); // sync_active_level_length
    bfrcr.set_fsdef(true); // frame_sync_definition
    bfrcr.set_fspol(false); // frame_sync_polarity
    bfrcr.set_fsoff(true); // frame_sync_offset
    sai.bfrcr.write(bfrcr);

    // configure slot
    let mut bslotr = sai::Bslotr::default();
    bslotr.set_fboff(0); // first_bit_offset
    bslotr.set_slotsz(0b00); // slot_size DataSize
    bslotr.set_nbslot(4 - 1); // number_of_slots
    bslotr.set_sloten(1 << 1 | 1 << 3); // enable_slots
    sai.bslotr.write(bslotr);

    // Enable SAI peripheral block a to generate MCLK
    sai.acr1.update(|r| r.set_saiaen(true)); // audio_block_enable

    // Enable SAI peripheral block b
    sai.bcr1.update(|r| r.set_saiben(true)); // audio_block_enable
}

pub fn init_sai_2_pins(gpio: &mut Gpio) {
    use embedded::interfaces::gpio::{OutputType, OutputSpeed, AlternateFunction, Resistor};
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;

    // block A (master)
    let sai2_fs_a = (PortI, Pin7);
    let sai2_sck_a = (PortI, Pin5);
    let sai2_sd_a = (PortI, Pin6);
    let sai2_mclk_a = (PortI, Pin4);
    // block B (synchronous slave)
    let sai2_sd_b = (PortG, Pin10);

    let pins = [sai2_fs_a, sai2_sck_a, sai2_sd_a, sai2_mclk_a, sai2_sd_b];
    gpio.to_alternate_function_all(&pins,
                                   AlternateFunction::AF10,
                                   OutputType::PushPull,
                                   OutputSpeed::High,
                                   Resistor::NoPull)
        .unwrap();
}
