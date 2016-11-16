use svd_board::rcc::Rcc;
use svd_board::sai1;
use svd_board::sai2::Sai2;
use gpio::{self, GpioController};
use i2c;
use system_clock;
use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};

const WM8994_ADDRESS: i2c::Address = i2c::Address::U7(0b0011010);

pub fn init_wm8994(i2c_3: &mut i2c::I2C) -> Result<(), i2c::Error> {
    i2c_3.connect(WM8994_ADDRESS, |session| {
        // read and check device family ID
        assert_eq!(session.register16(0)?.read_u16::<BigEndian>()?, 0x8994);
        // reset device
        session.register16(0)?.write_u16::<BigEndian>(0)?;

        // wm8994 Errata Work-Arounds
        session.register16(0x102)?.write_u16::<BigEndian>(0x0003)?;
        session.register16(0x817)?.write_u16::<BigEndian>(0x0000)?;
        session.register16(0x102)?.write_u16::<BigEndian>(0x0000)?;

        // Enable VMID soft start (fast), Start-up Bias Current Enabled
        session.register16(0x39)?.write_u16::<BigEndian>(0x006C)?;

        // Enable bias generator, Enable VMID
        session.register16(0x01)?.write_u16::<BigEndian>(0x0003)?;

        system_clock::wait(50);

        // INPUT_DEVICE_DIGITAL_MICROPHONE_2 :

        // Enable AIF1ADC2 (Left), Enable AIF1ADC2 (Right)
        // Enable DMICDAT2 (Left), Enable DMICDAT2 (Right)
        // Enable Left ADC, Enable Right ADC
        session.register16(0x04)?.write_u16::<BigEndian>(0x0C30)?;

        // Enable AIF1 DRC2 Signal Detect & DRC in AIF1ADC2 Left/Right Timeslot 1
        session.register16(0x450)?.write_u16::<BigEndian>(0x00DB)?;

        // Disable IN1L, IN1R, IN2L, IN2R, Enable Thermal sensor & shutdown
        session.register16(0x02)?.write_u16::<BigEndian>(0x6000)?;

        // Enable the DMIC2(Left) to AIF1 Timeslot 1 (Left) mixer path
        session.register16(0x608)?.write_u16::<BigEndian>(0x0002)?;

        // Enable the DMIC2(Right) to AIF1 Timeslot 1 (Right) mixer path
        session.register16(0x609)?.write_u16::<BigEndian>(0x0002)?;

        // GPIO1 pin configuration GP1_DIR = output, GP1_FN = AIF1 DRC2 signal detect
        session.register16(0x700)?.write_u16::<BigEndian>(0x000E)?;

        // Clock Configurations

        // AIF1 Sample Rate = 16 (KHz), ratio=256
        session.register16(0x210)?.write_u16::<BigEndian>(0x0033)?;

        // AIF1 Word Length = 16-bits, AIF1 Format = I2S (Default Register Value)
        session.register16(0x300)?.write_u16::<BigEndian>(0x4010)?;

        // slave mode
        session.register16(0x302)?.write_u16::<BigEndian>(0x0000)?;

        // Enable the DSP processing clock for AIF1, Enable the core clock
        session.register16(0x208)?.write_u16::<BigEndian>(0x000A)?;

        // Enable AIF1 Clock, AIF1 Clock Source = MCLK1 pin
        session.register16(0x200)?.write_u16::<BigEndian>(0x0001)?;

        // Enable Microphone bias 1 generator, Enable VMID
        session.register16(0x01)?.write_u16::<BigEndian>(0x0013)?;

        // ADC oversample enable
        session.register16(0x620)?.write_u16::<BigEndian>(0x0002)?;

        // AIF ADC2 HPF enable, HPF cut = voice mode 1 fc=127Hz at fs=8kHz
        session.register16(0x411)?.write_u16::<BigEndian>(0x3800)?;

        // set volume

        let convertedvol = 239; // 100(+17.625dB)

        // Left AIF1 ADC1 volume
        session.register16(0x400)?.write_u16::<BigEndian>(convertedvol | 0x100)?;

        // Right AIF1 ADC1 volume
        session.register16(0x401)?.write_u16::<BigEndian>(convertedvol | 0x100)?;

        // Left AIF1 ADC2 volume
        session.register16(0x404)?.write_u16::<BigEndian>(convertedvol | 0x100)?;

        // Right AIF1 ADC2 volume
        session.register16(0x405)?.write_u16::<BigEndian>(convertedvol | 0x100)?;

        Ok(())
    })
}

pub fn init_sai_2(sai: &mut Sai2, rcc: &mut Rcc) {
    let audio_frequency = 16000;

    // disable block a and block b
    sai.acr1.update(|r| r.set_saiaen(false)); // audio_block_enable
    sai.bcr1.update(|r| r.set_saiben(false)); // audio_block_enable
    while sai.acr1.read().saiaen() {}
    while sai.bcr1.read().saiben() {}

    // enable sai2 clock
    rcc.apb2enr.update(|r| r.set_sai2en(true));

    // Disabled All interrupt and clear all the flag
    sai.bim.write(sai1::Bim::reset_value());
    let mut clear_all_flags = sai1::Bclrfr::reset_value();
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

    let mut acr1 = sai1::Acr1::reset_value();
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
    let mut acr2 = sai1::Acr2::reset_value();
    acr2.set_fth(0b001); // fifo_threshold QuarterFifo
    acr2.set_tris(false); // tristate_management
    acr2.set_comp(0b00); // companding_mode None
    sai.acr2.write(acr2);

    // configure frame
    let mut afrcr = sai1::Afrcr::reset_value();
    afrcr.set_frl(64 - 1); // frame_length
    afrcr.set_fsall(32 - 1); // sync_active_level_length
    afrcr.set_fsdef(true); // frame_sync_definition
    afrcr.set_fspol(false); // frame_sync_polarity
    afrcr.set_fsoff(true); // frame_sync_offset
    sai.afrcr.write(afrcr);

    // configure slot
    let mut aslotr = sai1::Aslotr::reset_value();
    aslotr.set_fboff(0); // first_bit_offset
    aslotr.set_slotsz(0b00); // slot_size DataSize
    aslotr.set_nbslot(4 - 1); // number_of_slots
    aslotr.set_sloten(1 << 1 | 1 << 3); // enable_slots
    sai.aslotr.write(aslotr);

    // Initialize SAI2 block B in SLAVE RX synchronous from SAI2 block A

    // configure cr1
    let mut bcr1 = sai1::Bcr1::reset_value();
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
    let mut bcr2 = sai1::Bcr2::reset_value();
    bcr2.set_fth(0b001); // fifo_threshold QuarterFifo
    bcr2.set_tris(false); // tristate_management
    bcr2.set_comp(0b00); // companding_mode None
    sai.bcr2.write(bcr2);

    // configure frame
    let mut bfrcr = sai1::Bfrcr::reset_value();
    bfrcr.set_frl(64 - 1); // frame_length
    bfrcr.set_fsall(32 - 1); // sync_active_level_length
    bfrcr.set_fsdef(true); // frame_sync_definition
    bfrcr.set_fspol(false); // frame_sync_polarity
    bfrcr.set_fsoff(true); // frame_sync_offset
    sai.bfrcr.write(bfrcr);

    // configure slot
    let mut bslotr = sai1::Bslotr::reset_value();
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

pub fn init_sai_2_pins(gpio: &mut GpioController) {
    // block A (master)
    let sai2_fs_a = gpio.pins.i.7.take().unwrap();
    let sai2_sck_a = gpio.pins.i.5.take().unwrap();
    let sai2_sd_a = gpio.pins.i.6.take().unwrap();
    let sai2_mclk_a = gpio.pins.i.4.take().unwrap();
    // block B (synchronous slave)
    let sai2_sd_b = gpio.pins.g.10.take().unwrap();

    let t = gpio::Type::PushPull;
    let s = gpio::Speed::High;
    let a = gpio::AlternateFunction::AF10;
    let r = gpio::Resistor::NoPull;

    gpio.to_alternate_function(sai2_fs_a, t, s, a, r);
    gpio.to_alternate_function(sai2_sck_a, t, s, a, r);
    gpio.to_alternate_function(sai2_sd_a, t, s, a, r);
    gpio.to_alternate_function(sai2_mclk_a, t, s, a, r);
    gpio.to_alternate_function(sai2_sd_b, t, s, a, r);
}
