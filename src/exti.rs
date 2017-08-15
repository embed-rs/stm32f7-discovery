use board::embedded::interfaces::gpio::Port;
use board;
use volatile::ReadWrite;


pub struct Exti {
    exti: &'static mut board::exti::Exti,
    lines_used: [bool; 24],
}

impl Exti {

    pub fn new(exti: &'static mut board::exti::Exti) -> Exti {
        Exti {
            exti: exti,
            lines_used: [false; 24],
        }
    }

    pub fn register(&mut self, exti_line: ExtiLine, edge_detection: EdgeDetection, syscfg: &mut board::syscfg::Syscfg) -> Result<ExtiHandle, LineAlreadyUsedError> {
        
        macro_rules! set_registers {
            ($number:expr, $resyscfg:ident, $multi:ident, $imr:ident, $tr:ident, $port:ident) => {{
                if self.lines_used[$number] {
                    return Err(LineAlreadyUsedError(exti_line));
                }

                self.lines_used[$number] = true;

                use self::Port::*;

                match $port {
                    PortA => syscfg.$resyscfg.update(|r| r.$multi(0)),
                    PortB => syscfg.$resyscfg.update(|r| r.$multi(1)),
                    PortC => syscfg.$resyscfg.update(|r| r.$multi(2)),
                    PortD => syscfg.$resyscfg.update(|r| r.$multi(3)),
                    PortE => syscfg.$resyscfg.update(|r| r.$multi(4)),
                    PortF => syscfg.$resyscfg.update(|r| r.$multi(5)),
                    PortG => syscfg.$resyscfg.update(|r| r.$multi(6)),
                    PortH => syscfg.$resyscfg.update(|r| r.$multi(7)),
                    PortI => syscfg.$resyscfg.update(|r| r.$multi(8)),
                    PortJ => syscfg.$resyscfg.update(|r| r.$multi(9)),
                    PortK => syscfg.$resyscfg.update(|r| r.$multi(110)),
                }

                self.exti.imr.update(|r| r.$imr(true));

                use self::EdgeDetection::*;

                match edge_detection {
                    RisingEdge => {
                        self.exti.rtsr.update(|r| r.$tr(true));
                        self.exti.ftsr.update(|r| r.$tr(false));
                    },
                    FallingEdge => {
                        self.exti.ftsr.update(|r| r.$tr(true));
                        self.exti.rtsr.update(|r| r.$tr(false));
                    },
                    BothEdges => {
                        self.exti.rtsr.update(|r| r.$tr(true));
                        self.exti.ftsr.update(|r| r.$tr(true));
                    },
                }
            }};
            ($number:expr, $imr:ident, $tr:ident) => {{
                if self.lines_used[$number] {
                    return Err(LineAlreadyUsedError(exti_line));
                }

                self.lines_used[$number] = true;

                self.exti.imr.update(|r| r.$imr(true));

                use self::EdgeDetection::*;

                match edge_detection {
                    RisingEdge => {
                        self.exti.rtsr.update(|r| r.$tr(true));
                        self.exti.ftsr.update(|r| r.$tr(false));
                    },
                    FallingEdge => {
                        self.exti.ftsr.update(|r| r.$tr(true));
                        self.exti.rtsr.update(|r| r.$tr(false));
                    },
                    BothEdges => {
                        self.exti.rtsr.update(|r| r.$tr(true));
                        self.exti.ftsr.update(|r| r.$tr(true));
                    },
                }
            }};
        }

        use self::ExtiLine::*;

        match exti_line {
            Line0(port) => set_registers!(0, exticr1, set_exti0, set_mr0, set_tr0, port),
            Line1(port) => set_registers!(1, exticr1, set_exti1, set_mr1, set_tr1, port),
            Line2(port) => set_registers!(2, exticr1, set_exti2, set_mr2, set_tr2, port),
            Line3(port) => set_registers!(3, exticr1, set_exti3, set_mr3, set_tr3, port),
            Line4(port) => set_registers!(4, exticr2, set_exti4, set_mr4, set_tr4, port),
            Line5(port) => set_registers!(5, exticr2, set_exti5, set_mr5, set_tr5, port),
            Line6(port) => set_registers!(6, exticr2, set_exti6, set_mr6, set_tr6, port),
            Line7(port) => set_registers!(7, exticr2, set_exti7, set_mr7, set_tr7, port),
            Line8(port) => set_registers!(8, exticr3, set_exti8, set_mr8, set_tr8, port),
            Line9(port) => set_registers!(9, exticr3, set_exti9, set_mr9, set_tr9, port),
            Line10(port) => set_registers!(10, exticr3, set_exti10, set_mr10, set_tr10, port),
            Line11(port) => set_registers!(11, exticr3, set_exti11, set_mr11, set_tr11, port),
            Line12(port) => set_registers!(12, exticr4, set_exti12, set_mr12, set_tr12, port),
            Line13(port) => set_registers!(13, exticr4, set_exti13, set_mr13, set_tr13, port),
            Line14(port) => set_registers!(14, exticr4, set_exti14, set_mr14, set_tr14, port),
            Line15(port) => set_registers!(15, exticr4, set_exti15, set_mr15, set_tr15, port),
            Line16 => set_registers!(16, set_mr16, set_tr16),
            Line17 => set_registers!(17, set_mr17, set_tr17),
            Line18 => set_registers!(18, set_mr18, set_tr18),
            Line19 => set_registers!(19, set_mr19, set_tr19),
            Line20 => set_registers!(20, set_mr20, set_tr20),
            Line21 => set_registers!(21, set_mr21, set_tr21),
            Line22 => set_registers!(22, set_mr22, set_tr22),
            //Line23 => pr.set_pr23(value),
            _ => unreachable!(), 
        }

        let handle = ExtiHandle {
            exti_line: exti_line,
            pr: PrRef(&mut self.exti.pr),
        };

        Ok(handle)
    }

    pub fn unregister(&mut self, exti_handle: ExtiHandle) {
        
        use self::ExtiLine::*;
        
        match exti_handle.exti_line {
            Line0(_) => self.exti.imr.update(|r| r.set_mr0(false)),
            Line1(_) => self.exti.imr.update(|r| r.set_mr1(false)),
            Line2(_) => self.exti.imr.update(|r| r.set_mr2(false)),
            Line3(_) => self.exti.imr.update(|r| r.set_mr3(false)),
            Line4(_) => self.exti.imr.update(|r| r.set_mr4(false)),
            Line5(_) => self.exti.imr.update(|r| r.set_mr5(false)),
            Line6(_) => self.exti.imr.update(|r| r.set_mr6(false)),
            Line7(_) => self.exti.imr.update(|r| r.set_mr7(false)),
            Line8(_) => self.exti.imr.update(|r| r.set_mr8(false)),
            Line9(_) => self.exti.imr.update(|r| r.set_mr9(false)),
            Line10(_) => self.exti.imr.update(|r| r.set_mr10(false)),
            Line11(_) => self.exti.imr.update(|r| r.set_mr11(false)),
            Line12(_) => self.exti.imr.update(|r| r.set_mr12(false)),
            Line13(_) => self.exti.imr.update(|r| r.set_mr13(false)),
            Line14(_) => self.exti.imr.update(|r| r.set_mr14(false)),
            Line15(_) => self.exti.imr.update(|r| r.set_mr15(false)),
            Line16 => self.exti.imr.update(|r| r.set_mr16(false)),
            Line17 => self.exti.imr.update(|r| r.set_mr17(false)),
            Line18 => self.exti.imr.update(|r| r.set_mr18(false)),
            Line19 => self.exti.imr.update(|r| r.set_mr19(false)),
            Line20 => self.exti.imr.update(|r| r.set_mr20(false)),
            Line21 => self.exti.imr.update(|r| r.set_mr21(false)),
            Line22 => self.exti.imr.update(|r| r.set_mr22(false)),
            //Line23 => self.exti.imr.update(|r| r.set_mr23),
            _ => unreachable!(),
        }
    }


}

#[derive(Debug)]
pub struct LineAlreadyUsedError(ExtiLine);

pub struct ExtiHandle {
    pub exti_line: ExtiLine,
    pub pr: PrRef,
}

impl ExtiHandle {
    pub fn clear_pending_state(&mut self) {
        self.pr.set(self.exti_line, true);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtiLine {
    Line0(Port),
    Line1(Port),
    Line2(Port),
    Line3(Port),
    Line4(Port),
    Line5(Port),
    Line6(Port),
    Line7(Port),
    Line8(Port),
    Line9(Port),
    Line10(Port),
    Line11(Port),
    Line12(Port),
    Line13(Port),
    Line14(Port),
    Line15(Port),
    Line16,
    Line17,
    Line18,
    Line19,
    Line20,
    Line21,
    Line22,
    Line23,
}

pub enum EdgeDetection {
    RisingEdge,
    FallingEdge,
    BothEdges,
}

pub struct PrRef(pub *mut ReadWrite<board::exti::Pr>);

unsafe impl Send for PrRef {}

impl PrRef{
    fn set(&self, exti_line: ExtiLine, value: bool) {
        use board::exti::Pr;
        let mut pr = Pr::default();

        use self::ExtiLine::*;

        match exti_line {
            Line0(_) => pr.set_pr0(value),
            Line1(_) => pr.set_pr1(value),
            Line2(_) => pr.set_pr2(value),
            Line3(_) => pr.set_pr3(value),
            Line4(_) => pr.set_pr4(value),
            Line5(_) => pr.set_pr5(value),
            Line6(_) => pr.set_pr6(value),
            Line7(_) => pr.set_pr7(value),
            Line8(_) => pr.set_pr8(value),
            Line9(_) => pr.set_pr9(value),
            Line10(_) => pr.set_pr10(value),
            Line11(_) => pr.set_pr11(value),
            Line12(_) => pr.set_pr12(value),
            Line13(_) => pr.set_pr13(value),
            Line14(_) => pr.set_pr14(value),
            Line15(_) => pr.set_pr15(value),
            Line16 => pr.set_pr16(value),
            Line17 => pr.set_pr17(value),
            Line18 => pr.set_pr18(value),
            Line19 => pr.set_pr19(value),
            Line20 => pr.set_pr20(value),
            Line21 => pr.set_pr21(value),
            Line22 => pr.set_pr22(value),
            //Line23 => pr.set_pr23(value),
            _ => unreachable!(),
        }
        // Data Race? I think there is no Data race, because when you write to the register bit a 1 the corresponding bit is cleared... so no read and than write again... only write.
        // Writing a 0 to a register bit does not effect the current state.
        unsafe {
            (&mut *self.0).write(pr);
        };
    }
}



