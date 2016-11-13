use svd_board::rcc::Rcc;
use svd_board::ltdc::Ltdc;
use gpio::{self, GpioController, GpioWrite};
use super::Lcd;

pub fn init(ltdc: &'static mut Ltdc, rcc: &mut Rcc, gpio: &mut GpioController) -> Lcd {
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

    rcc.pllsaicfgr.update(|r| {
        r.set_pllsain(192);
        r.set_pllsair(5);
    });

    // set division factor for LCD_CLK
    rcc.dkcfgr1.update(|r| {
        r.set_pllsaidivr(0b01 /* = 4 */)
    });

    // enable PLLSAI clock
    rcc.cr.update(|r| r.set_pllsaion(true));
    while !rcc.cr.read().pllsairdy() {}

    // configure the HS, VS, DE and PC polarity
    ltdc.gcr.update(|r| {
        r.set_pcpol(false);
        r.set_depol(false);
        r.set_hspol(false);
        r.set_vspol(false);
    });

    // set synchronization size
    ltdc.sscr.update(|r| {
        r.set_hsw(41 - 1); // horizontal_sync_width
        r.set_vsh(10 - 1); // vertical_sync_height
    });

    // set accumulated back porch
    ltdc.bpcr.update(|r| {
        r.set_ahbp(41 + 13 - 1); // accumulated_horizontal_back_porch
        r.set_avbp(10 + 2 - 1); // accumulated_vertical_back_porch
    });

    // set accumulated active width
    ltdc.awcr.update(|r| {
        r.set_aaw(480 + 41 + 13 - 1); // accumulated_active_width
        r.set_aah(272 + 10 + 2 - 1); // accumulated_active_height
    });

    // set total width
    ltdc.twcr.update(|r| {
        r.set_totalw(480 + 41 + 13 + 32 - 1); // total_width
        r.set_totalh(272 + 10 + 2 + 2 - 1); // total_height
    });

    // set background color
    ltdc.bccr.update(|r| r.set_bc(0x0000ff)); // background_color blue


    // enable the transfer error interrupt and the FIFO underrun interrupt
    ltdc.ier.update(|r| {
        r.set_terrie(true); // TRANSFER_ERROR_INTERRUPT_ENABLE
        r.set_fuie(true); // FIFO_UNDERRUN_INTERRUPT_ENABLE
    });

    // enable LTDC
    ltdc.gcr.update(|r| r.set_ltdcen(true));

    // configure layers

    // configure horizontal start and stop position
    ltdc.l1whpcr.update(|r| {
        r.set_whstpos(0 + 41 + 13); // window_horizontal_start_position
        r.set_whsppos(480 + 41 + 13 - 1); // window_horizontal_stop_position
    });
    ltdc.l2whpcr.update(|r| {
        r.set_whstpos(0 + 41 + 13); // window_horizontal_start_position
        r.set_whsppos(480 + 41 + 13 - 1); // window_horizontal_stop_position
    });

    // configure vertical start and stop position
    ltdc.l1wvpcr.update(|r| {
        r.set_wvstpos(0 + 10 + 2); // window_vertical_start_position
        r.set_wvsppos(272 + 10 + 2 - 1); // window_vertical_stop_position
    });
    ltdc.l2wvpcr.update(|r| {
        r.set_wvstpos(0 + 10 + 2); // window_vertical_start_position
        r.set_wvsppos(272 + 10 + 2 - 1); // window_vertical_stop_position
    });

    // specify pixed format
    ltdc.l1pfcr.update(|r| r.set_pf(0b010)); // set_pixel_format to RGB565
    ltdc.l2pfcr.update(|r| r.set_pf(0b010)); // set_pixel_format to RGB565

    // configure default color values
    ltdc.l1dccr.update(|r| {
        r.set_dcalpha(0);
        r.set_dcred(0);
        r.set_dcgreen(0);
        r.set_dcblue(0);
    });
    ltdc.l2dccr.update(|r| {
        r.set_dcalpha(0);
        r.set_dcred(0);
        r.set_dcgreen(0);
        r.set_dcblue(0);
    });

    // specify constant alpha value
    ltdc.l1cacr.update(|r| r.set_consta(200)); // constant_alpha
    ltdc.l2cacr.update(|r| r.set_consta(50)); // constant_alpha

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
    const SDRAM_START: u32 = 0xC000_0000;
    ltdc.l1cfbar.update(|r| r.set_cfbadd(SDRAM_START));
    ltdc.l2cfbar.update(|r| r.set_cfbadd(SDRAM_START + 480 * 272 * 2));

    // configure color frame buffer line length and pitch
    ltdc.l1cfblr.update(|r| {
        r.set_cfbp(480 * 2); // pitch
        r.set_cfbll(480 * 2 + 3); // line_length
    });
    ltdc.l2cfblr.update(|r| {
        r.set_cfbp(480 * 2); // pitch
        r.set_cfbll(480 * 2 + 3); // line_length
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
        next_pixel: 0,
        next_col: 0,
        prev_value: (0, 0),
    }
}

pub fn init_pins(gpio: &mut GpioController) -> (GpioWrite<gpio::Pin12>, GpioWrite<gpio::Pin3>) {
    use gpio::{Type, Speed, AlternateFunction, Resistor};

    // Red
    let r0 = gpio.pins.i.15.take().unwrap();
    let r1 = gpio.pins.j.0.take().unwrap();
    let r2 = gpio.pins.j.1.take().unwrap();
    let r3 = gpio.pins.j.2.take().unwrap();
    let r4 = gpio.pins.j.3.take().unwrap();
    let r5 = gpio.pins.j.4.take().unwrap();
    let r6 = gpio.pins.j.5.take().unwrap();
    let r7 = gpio.pins.j.6.take().unwrap();

    // Green
    let g0 = gpio.pins.j.7.take().unwrap();
    let g1 = gpio.pins.j.8.take().unwrap();
    let g2 = gpio.pins.j.9.take().unwrap();
    let g3 = gpio.pins.j.10.take().unwrap();
    let g4 = gpio.pins.j.11.take().unwrap();
    let g5 = gpio.pins.k.0.take().unwrap();
    let g6 = gpio.pins.k.1.take().unwrap();
    let g7 = gpio.pins.k.2.take().unwrap();

    // Blue
    let b0 = gpio.pins.e.4.take().unwrap();
    let b1 = gpio.pins.j.13.take().unwrap();
    let b2 = gpio.pins.j.14.take().unwrap();
    let b3 = gpio.pins.j.15.take().unwrap();
    let b4 = gpio.pins.g.12.take().unwrap();
    let b5 = gpio.pins.k.4.take().unwrap();
    let b6 = gpio.pins.k.5.take().unwrap();
    let b7 = gpio.pins.k.6.take().unwrap();

    let clk = gpio.pins.i.14.take().unwrap();
    let data_enable = gpio.pins.k.7.take().unwrap();
    let hsync = gpio.pins.i.10.take().unwrap();
    let vsync = gpio.pins.i.9.take().unwrap();

    let t = Type::PushPull;
    let s = Speed::High;
    let a = AlternateFunction::AF14;
    let r = Resistor::NoPull;

    gpio.to_alternate_function(r0, t, s, a, r);
    gpio.to_alternate_function(r1, t, s, a, r);
    gpio.to_alternate_function(r2, t, s, a, r);
    gpio.to_alternate_function(r3, t, s, a, r);
    gpio.to_alternate_function(r4, t, s, a, r);
    gpio.to_alternate_function(r5, t, s, a, r);
    gpio.to_alternate_function(r6, t, s, a, r);
    gpio.to_alternate_function(r7, t, s, a, r);
    gpio.to_alternate_function(g0, t, s, a, r);
    gpio.to_alternate_function(g1, t, s, a, r);
    gpio.to_alternate_function(g2, t, s, a, r);
    gpio.to_alternate_function(g3, t, s, a, r);
    gpio.to_alternate_function(g4, t, s, a, r);
    gpio.to_alternate_function(g5, t, s, a, r);
    gpio.to_alternate_function(g6, t, s, a, r);
    gpio.to_alternate_function(g7, t, s, a, r);
    gpio.to_alternate_function(b0, t, s, a, r);
    gpio.to_alternate_function(b1, t, s, a, r);
    gpio.to_alternate_function(b2, t, s, a, r);
    gpio.to_alternate_function(b3, t, s, a, r);
    gpio.to_alternate_function(b4, t, s, a, r);
    gpio.to_alternate_function(b5, t, s, a, r);
    gpio.to_alternate_function(b6, t, s, a, r);
    gpio.to_alternate_function(b7, t, s, a, r);
    gpio.to_alternate_function(clk, t, s, a, r);
    gpio.to_alternate_function(data_enable, t, s, a, r);
    gpio.to_alternate_function(hsync, t, s, a, r);
    gpio.to_alternate_function(vsync, t, s, a, r);

    // Display control
    let display_enable_pin = gpio.pins.i.12.take().unwrap();
    let backlight_pin = gpio.pins.k.3.take().unwrap();

    let display_enable = gpio.to_output(display_enable_pin,
                                        Type::PushPull,
                                        Speed::Low,
                                        Resistor::PullDown);
    let backlight = gpio.to_output(backlight_pin,
                                   Type::PushPull,
                                   Speed::Low,
                                   Resistor::PullDown);

    (display_enable, backlight)

}
