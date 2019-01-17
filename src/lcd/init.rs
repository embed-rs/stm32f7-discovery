use super::Lcd;
use stm32f7::stm32f7x6::{LTDC, RCC};

/// Initializes the LCD controller.
///
/// The SDRAM must be initialized before this function is called. See the
/// [`init_sdram`] function for more information.
///
/// [`init_sdram`]: crate::init::init_sdram
pub fn init<'a>(ltdc: &'a mut LTDC, rcc: &mut RCC) -> Lcd<'a> {
    use crate::lcd::{self, LAYER_1_START, LAYER_2_START};
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

    Lcd::new(ltdc)
}
