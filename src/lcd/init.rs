use board::rcc::Rcc;
use board::ltdc::Ltdc;
use embedded::interfaces::gpio::{Gpio, OutputPin};
use super::{Lcd, LAYER_1_START, LAYER_2_START};

pub fn init(ltdc: &'static mut Ltdc, rcc: &mut Rcc, gpio: &mut Gpio) -> Lcd {
    // init gpio pins
    let (mut display_enable, mut backlight_enable) = init_pins(gpio);

    // enable LTDC and DMA2D clocks
    rcc.ahb1enr.update(|r| r.set_dma2den(true));
    rcc.apb2enr.update(|r| r.set_ltdcen(true));

    // disable LTDC
    ltdc.gcr.update(|r| r.set_ltdcen(false));

    // disable PLLSAI clock
    rcc.cr.update(|r| r.set_pllsaion(false));
    while rcc.cr.read().pllsairdy() {}

    rcc.pllsaicfgr
        .update(|r| {
                    r.set_pllsain(192);
                    r.set_pllsair(5);
                });

    // set division factor for LCD_CLK
    rcc.dkcfgr1
        .update(|r| {
                    r.set_pllsaidivr(0b01 /* = 4 */)
                });

    // enable PLLSAI clock
    rcc.cr.update(|r| r.set_pllsaion(true));
    while !rcc.cr.read().pllsairdy() {}

    // configure the HS, VS, DE and PC polarity
    ltdc.gcr
        .update(|r| {
                    r.set_pcpol(false);
                    r.set_depol(false);
                    r.set_hspol(false);
                    r.set_vspol(false);
                });

    // set synchronization size
    ltdc.sscr
        .update(|r| {
                    r.set_hsw(41 - 1); // horizontal_sync_width
                    r.set_vsh(10 - 1); // vertical_sync_height
                });

    // set accumulated back porch
    ltdc.bpcr
        .update(|r| {
                    r.set_ahbp(41 + 13 - 1); // accumulated_horizontal_back_porch
                    r.set_avbp(10 + 2 - 1); // accumulated_vertical_back_porch
                });

    // set accumulated active width
    ltdc.awcr
        .update(|r| {
                    r.set_aaw(480 + 41 + 13 - 1); // accumulated_active_width
                    r.set_aah(272 + 10 + 2 - 1); // accumulated_active_height
                });

    // set total width
    ltdc.twcr
        .update(|r| {
                    r.set_totalw(480 + 41 + 13 + 32 - 1); // total_width
                    r.set_totalh(272 + 10 + 2 + 2 - 1); // total_height
                });

    // set background color
    ltdc.bccr.update(|r| r.set_bc(0x0000ff)); // background_color blue


    // enable the transfer error interrupt and the FIFO underrun interrupt
    ltdc.ier
        .update(|r| {
                    r.set_terrie(true); // TRANSFER_ERROR_INTERRUPT_ENABLE
                    r.set_fuie(true); // FIFO_UNDERRUN_INTERRUPT_ENABLE
                });

    // enable LTDC
    ltdc.gcr.update(|r| r.set_ltdcen(true));

    // configure layers

    // configure horizontal start and stop position
    ltdc.l1whpcr
        .update(|r| {
                    r.set_whstpos(0 + 41 + 13); // window_horizontal_start_position
                    r.set_whsppos(480 + 41 + 13 - 1); // window_horizontal_stop_position
                });
    ltdc.l2whpcr
        .update(|r| {
                    r.set_whstpos(0 + 41 + 13); // window_horizontal_start_position
                    r.set_whsppos(480 + 41 + 13 - 1); // window_horizontal_stop_position
                });

    // configure vertical start and stop position
    ltdc.l1wvpcr
        .update(|r| {
                    r.set_wvstpos(0 + 10 + 2); // window_vertical_start_position
                    r.set_wvsppos(272 + 10 + 2 - 1); // window_vertical_stop_position
                });
    ltdc.l2wvpcr
        .update(|r| {
                    r.set_wvstpos(0 + 10 + 2); // window_vertical_start_position
                    r.set_wvsppos(272 + 10 + 2 - 1); // window_vertical_stop_position
                });

    // specify pixed format
    ltdc.l1pfcr.update(|r| r.set_pf(0b000)); // set_pixel_format to ARGB8888
    ltdc.l2pfcr.update(|r| r.set_pf(0b111)); // set_pixel_format to AL88

    // configure default color values
    ltdc.l1dccr
        .update(|r| {
                    r.set_dcalpha(0);
                    r.set_dcred(0);
                    r.set_dcgreen(0);
                    r.set_dcblue(0);
                });
    ltdc.l2dccr
        .update(|r| {
                    r.set_dcalpha(0);
                    r.set_dcred(0);
                    r.set_dcgreen(0);
                    r.set_dcblue(0);
                });

    // specify constant alpha value
    ltdc.l1cacr.update(|r| r.set_consta(255)); // constant_alpha
    ltdc.l2cacr.update(|r| r.set_consta(255)); // constant_alpha

    // specify blending factors
    ltdc.l1bfcr.update(|r| {
        r.set_bf1(0b110); // set_blending_factor_1 to PixelAlphaTimesConstantAlpha
        r.set_bf2(0b111); // set_blending_factor_2 to OneMinusPixelAlphaTimesConstantAlpha
    });
    ltdc.l2bfcr.update(|r| {
        r.set_bf1(0b110); // set_blending_factor_1 to PixelAlphaTimesConstantAlpha
        r.set_bf2(0b111); // set_blending_factor_2 to OneMinusPixelAlphaTimesConstantAlpha
    });

    // configure color frame buffer start address
    ltdc.l1cfbar.update(|r| r.set_cfbadd(LAYER_1_START as u32));
    ltdc.l2cfbar.update(|r| r.set_cfbadd(LAYER_2_START as u32));

    // configure color frame buffer line length and pitch
    ltdc.l1cfblr
        .update(|r| {
                    r.set_cfbp(480 * 4); // pitch
                    r.set_cfbll(480 * 4 + 3); // line_length
                });
    ltdc.l2cfblr
        .update(|r| {
                    r.set_cfbp(480 * 4); // pitch
                    r.set_cfbll(480 * 4 + 3); // line_length
                });

    // configure frame buffer line number
    ltdc.l1cfblnr.update(|r| r.set_cfblnbr(272)); // line_number
    ltdc.l2cfblnr.update(|r| r.set_cfblnbr(272)); // line_number

    // enable layers
    ltdc.l1cr.update(|r| r.set_len(true));
    ltdc.l2cr.update(|r| r.set_len(true));

    // reload shadow registers
    ltdc.srcr.update(|r| r.set_imr(true)); // IMMEDIATE_RELOAD

    // init DMA2D graphic



    // enable display and backlight
    display_enable.set(true);
    backlight_enable.set(true);

    // TODO
    //
    // Init LTDC layers */
    // TM_LCD_INT_InitLayers();
    // Init DMA2D GRAPHICS */
    // TM_DMA2DGRAPHIC_Init();
    // Set settings */
    // TM_INT_DMA2DGRAPHIC_SetConf(&DMA2DConf);
    // Enable LCD */
    // TM_LCD_DisplayOn();
    // Set layer 1 as active layer */
    // TM_LCD_SetLayer1();
    // TM_LCD_Fill(LCD_COLOR_WHITE);
    // TM_LCD_SetLayer2();
    // TM_LCD_Fill(LCD_COLOR_WHITE);
    // TM_LCD_SetLayer1();
    // Set layer 1 as active layer */
    // TM_LCD_SetLayer1Opacity(255);
    // TM_LCD_SetLayer2Opacity(0);
    //
    //

    Lcd {
        controller: ltdc,
        display_enable: display_enable,
        backlight_enable: backlight_enable,
        layer_1_in_use: false,
        layer_2_in_use: false,
    }
}

pub fn init_pins(gpio: &mut Gpio) -> (OutputPin, OutputPin) {
    use embedded::interfaces::gpio::Port::*;
    use embedded::interfaces::gpio::Pin::*;
    use embedded::interfaces::gpio::{OutputType, OutputSpeed, AlternateFunction, Resistor};

    // Red
    let r0 = (PortI, Pin15);
    let r1 = (PortJ, Pin0);
    let r2 = (PortJ, Pin1);
    let r3 = (PortJ, Pin2);
    let r4 = (PortJ, Pin3);
    let r5 = (PortJ, Pin4);
    let r6 = (PortJ, Pin5);
    let r7 = (PortJ, Pin6);

    // Green
    let g0 = (PortJ, Pin7);
    let g1 = (PortJ, Pin8);
    let g2 = (PortJ, Pin9);
    let g3 = (PortJ, Pin10);
    let g4 = (PortJ, Pin11);
    let g5 = (PortK, Pin0);
    let g6 = (PortK, Pin1);
    let g7 = (PortK, Pin2);

    // Blue
    let b0 = (PortE, Pin4);
    let b1 = (PortJ, Pin13);
    let b2 = (PortJ, Pin14);
    let b3 = (PortJ, Pin15);
    let b4 = (PortG, Pin12);
    let b5 = (PortK, Pin4);
    let b6 = (PortK, Pin5);
    let b7 = (PortK, Pin6);

    let clk = (PortI, Pin14);
    let data_enable = (PortK, Pin7);
    let hsync = (PortI, Pin10);
    let vsync = (PortI, Pin9);

    let pins = [r0,
                r1,
                r2,
                r3,
                r4,
                r5,
                r6,
                r7,
                g0,
                g1,
                g2,
                g3,
                g4,
                g5,
                g6,
                g7,
                b0,
                b1,
                b2,
                b3,
                b4,
                b5,
                b6,
                b7,
                clk,
                data_enable,
                hsync,
                vsync];
    gpio.to_alternate_function_all(&pins,
                                   AlternateFunction::AF14,
                                   OutputType::PushPull,
                                   OutputSpeed::High,
                                   Resistor::NoPull)
        .unwrap();

    // Display control
    let display_enable_pin = (PortI, Pin12);
    let backlight_pin = (PortK, Pin3);

    let display_enable = gpio.to_output(display_enable_pin,
                                        OutputType::PushPull,
                                        OutputSpeed::Low,
                                        Resistor::PullDown)
        .unwrap();
    let backlight = gpio.to_output(backlight_pin,
                                   OutputType::PushPull,
                                   OutputSpeed::Low,
                                   Resistor::PullDown)
        .unwrap();

    (display_enable, backlight)

}
