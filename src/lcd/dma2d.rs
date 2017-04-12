use board::dma2d;
use lcd::Color;

pub struct Dma2d<'a> {
    registers: &'a mut dma2d::Dma2d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    RegisterToMemory = 0b11,
    MemoryToMemory = 0b00,
    MemoryToMemoryWithPfc = 0b01,
    MemoryToMemoryWithBlending = 0b10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Pfc {
    Argb8888 = 0b0000,
    Rgb888 = 0b0001,
    Rgb565 = 0b0010,
    Argb1555 = 0b0011,
    Argb4444 = 0b0100,
    L8 = 0b0101,
    Al44 = 0b0110,
    Al88 = 0b0111,
    L4 = 0b1000,
    A8 = 0b1001,
    A4 = 0b1010,
}

impl<'a> Dma2d<'a> {
    pub fn new(dma2d: &'a mut dma2d::Dma2d) -> Self {
        Dma2d {
            registers: dma2d,
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.registers.cr.update(|r| r.set_mode(mode as u8));
    }

    /// Set output memory address
    pub fn set_out_addr(&mut self, addr: usize) {
        let mut omar = dma2d::Omar::default();
        omar.set_ma(addr as u32);
        self.registers.omar.write(omar);
    }

    pub fn set_out_color(&mut self, color: Color) {
         // output color
        let mut ocolr = dma2d::Ocolr::default();
        ocolr.set_aplha(color.alpha);
        ocolr.set_red(color.red);
        ocolr.set_green(color.green);
        ocolr.set_blue(color.blue);
        self.registers.ocolr.write(ocolr);
    }

    /// Set output line offset
    pub fn set_out_line_offset(&mut self, line_offset: u16) {
        // output offset
        let mut oor = dma2d::Oor::default();
        oor.set_lo(line_offset); // line offset
        self.registers.oor.write(oor);
    }

    /// Set out pixel frame conversion
    pub fn set_out_pfc(&mut self, o_pfc: Pfc) {
        // out PFC control
        let mut opfccr = dma2d::Opfccr::default();
        opfccr.set_cm(o_pfc as u8);
        self.registers.opfccr.write(opfccr);
    }

    /// Set foreground memory address
    pub fn set_fg_addr(&mut self, fg_addr: usize) {
        // foreground memory address
        let mut fgmar = dma2d::Fgmar::default();
        fgmar.set_ma(fg_addr as u32);
        self.registers.fgmar.write(fgmar);
    }

    pub fn set_fg_line_offset(&mut self, fg_line_offset: u16) {
         // foreground offset
        let mut fgor = dma2d::Fgor::default();
        fgor.set_lo(fg_line_offset); // line offset
        self.registers.fgor.write(fgor);
    }

    /// Set foreground pixel frame conversion
    pub fn set_fg_pfc(&mut self, fg_pfc: Pfc) {
        // foreground PFC control
        let mut fgpfccr = dma2d::Fgpfccr::default();
        fgpfccr.set_cm(fg_pfc as u8);
        self.registers.fgpfccr.write(fgpfccr);
    }

    /// Set foreground color
    pub fn set_fg_color(&mut self, fg_color: Color) {
        let mut fgcolr = dma2d::Fgcolr::default();
        fgcolr.set_red(fg_color.red);
        fgcolr.set_green(fg_color.green);
        fgcolr.set_blue(fg_color.blue);
        self.registers.fgcolr.write(fgcolr);

    }

    /// Set background memory address
    pub fn set_bg_addr(&mut self, bg_addr: usize) {
        // background memory address
        let mut bgmar = dma2d::Bgmar::default();
        bgmar.set_ma(bg_addr as u32);
        self.registers.bgmar.write(bgmar);
    }

    pub fn set_bg_line_offset(&mut self, bg_line_offset: u16) {
         // background offset
        let mut bgor = dma2d::Bgor::default();
        bgor.set_lo(bg_line_offset); // line offset
        self.registers.bgor.write(bgor);
    }

    /// Set background pixel frame conversion
    pub fn set_bg_pfc(&mut self, bg_pfc: Pfc) {
        // background PFC control
        let mut bgpfccr = dma2d::Bgpfccr::default();
        bgpfccr.set_cm(bg_pfc as u8);
        self.registers.bgpfccr.write(bgpfccr);
    }

    /// Set background color
    pub fn set_bg_color(&mut self, bg_color: Color) {
        let mut bgcolr = dma2d::Bgcolr::default();
        bgcolr.set_red(bg_color.red);
        bgcolr.set_green(bg_color.green);
        bgcolr.set_blue(bg_color.blue);
        self.registers.bgcolr.write(bgcolr);

    }

    pub fn set_line_config(&mut self, pixel_per_line: u16, number_of_lines: u16) {
        // number of lines
        let mut nlr = dma2d::Nlr::default();
        nlr.set_pl(pixel_per_line); // pixel per line
        nlr.set_nl(number_of_lines); // number of lines
        self.registers.nlr.write(nlr);
    }

    pub fn start(&mut self) {
         // set start bit
        self.registers.cr.update(|r| r.set_start(true));

        // wait for start bit reset
        while self.registers.cr.read().start() {}
    }

    pub fn fill_color(&mut self, addr: usize, pixel_per_line: u16, number_of_lines: u16,
        line_offset: u16, color: Color)
    {
        self.set_mode(Mode::RegisterToMemory);

        self.set_out_addr(addr);
        self.set_out_line_offset(line_offset);
        self.set_out_color(color);

        self.set_line_config(pixel_per_line, number_of_lines);

        self.start();
    }

    pub fn memory_to_memory_blending(&mut self,
        fg_addr: usize, fg_line_offset: u16, fg_pfc: Pfc, fg_color: Color,
        bg_addr: usize, bg_line_offset: u16, bg_pfc: Pfc,
        out_addr: usize, out_line_offset: u16,
        pixel_per_line: u16, number_of_lines: u16)
    {
        self.set_mode(Mode::MemoryToMemoryWithBlending);

        self.set_fg_addr(fg_addr);
        self.set_fg_line_offset(fg_line_offset);
        self.set_fg_pfc(fg_pfc);
        self.set_fg_color(fg_color);

        self.set_bg_addr(bg_addr);
        self.set_bg_line_offset(bg_line_offset);
        self.set_bg_pfc(bg_pfc);

        self.set_out_addr(out_addr);
        self.set_out_line_offset(out_line_offset);
        self.set_out_pfc(Pfc::Argb8888);

        self.set_line_config(pixel_per_line, number_of_lines);

        self.start();
    }

    pub fn test(&mut self) {
        use super::{LAYER_1_START, LAYER_2_START};

        let pixel_per_line = 100;
        let number_of_lines = 100;

        self.set_mode(Mode::MemoryToMemoryWithBlending);

        self.set_fg_addr(LAYER_2_START);
        self.set_fg_line_offset(480 - pixel_per_line);
        self.set_fg_pfc(Pfc::Argb4444);

        self.set_bg_addr(LAYER_1_START + 100 * 480 * 4 + 300 *4);
        self.set_bg_line_offset(480 - pixel_per_line);
        self.set_bg_pfc(Pfc::Argb8888);

        self.set_out_addr(LAYER_1_START + 170 * 480 * 4 + 2 * 4);
        self.set_out_line_offset(480 - pixel_per_line);
        self.set_out_pfc(Pfc::Argb8888);

        self.set_line_config(pixel_per_line, number_of_lines);

        self.start();
    }
}
