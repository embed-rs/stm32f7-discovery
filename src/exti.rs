use board::embedded::interfaces::gpio::Port;
use board::embedded::components::gpio::stm32f7::Pin;
use board::syscfg::Syscfg;
use board::exti;
use volatile::ReadWrite;
use bit_field::BitField;


pub struct Exti {
    exti: &'static mut exti::Exti,
    lines_used: u32,
}

impl Exti {
    pub fn new(exti: &'static mut exti::Exti) -> Exti {
        Exti {
            exti: exti,
            lines_used: 0,
        }
    }

    pub fn register(
        &mut self,
        exti_line: ExtiLine,
        edge_detection: EdgeDetection,
        syscfg: &mut Syscfg,
    ) -> Result<ExtiHandle, LineAlreadyUsedError> {
        macro_rules! set_registers {
            ($number:expr, $resyscfg:ident, $multi:ident, $imr:ident, $tr:ident, $port:ident) => {{
                if self.lines_used.get_bit($number) {
                    return Err(LineAlreadyUsedError(exti_line));
                }

                self.lines_used.set_bit($number, true);


                syscfg.$resyscfg.update(|r| r.$multi($port as u8));

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
                if self.lines_used.get_bit($number) {
                    return Err(LineAlreadyUsedError(exti_line));
                }

                self.lines_used.set_bit($number, true);

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
            Gpio(port, pin) => {
                use self::Pin::*;

                match pin {
                    Pin0 => set_registers!(0, exticr1, set_exti0, set_mr0, set_tr0, port),
                    Pin1 => set_registers!(1, exticr1, set_exti1, set_mr1, set_tr1, port),
                    Pin2 => set_registers!(2, exticr1, set_exti2, set_mr2, set_tr2, port),
                    Pin3 => set_registers!(3, exticr1, set_exti3, set_mr3, set_tr3, port),
                    Pin4 => set_registers!(4, exticr2, set_exti4, set_mr4, set_tr4, port),
                    Pin5 => set_registers!(5, exticr2, set_exti5, set_mr5, set_tr5, port),
                    Pin6 => set_registers!(6, exticr2, set_exti6, set_mr6, set_tr6, port),
                    Pin7 => set_registers!(7, exticr2, set_exti7, set_mr7, set_tr7, port),
                    Pin8 => set_registers!(8, exticr3, set_exti8, set_mr8, set_tr8, port),
                    Pin9 => set_registers!(9, exticr3, set_exti9, set_mr9, set_tr9, port),
                    Pin10 => set_registers!(10, exticr3, set_exti10, set_mr10, set_tr10, port),
                    Pin11 => set_registers!(11, exticr3, set_exti11, set_mr11, set_tr11, port),
                    Pin12 => set_registers!(12, exticr4, set_exti12, set_mr12, set_tr12, port),
                    Pin13 => set_registers!(13, exticr4, set_exti13, set_mr13, set_tr13, port),
                    Pin14 => set_registers!(14, exticr4, set_exti14, set_mr14, set_tr14, port),
                    Pin15 => set_registers!(15, exticr4, set_exti15, set_mr15, set_tr15, port),
                }
            }
            PvdOutput => set_registers!(16, set_mr16, set_tr16),
            RtcAlarm => set_registers!(17, set_mr17, set_tr17),
            UsbOtgFsWakeup => set_registers!(18, set_mr18, set_tr18),
            EthernetWakeup => set_registers!(19, set_mr19, set_tr19),
            UsbOtgHsWakeup => set_registers!(20, set_mr20, set_tr20),
            RtcTamperAndTimeStamp => set_registers!(21, set_mr21, set_tr21),
            RtcWakeup => set_registers!(22, set_mr22, set_tr22),
            // Last line is missing in embedded_stm32f7
            Lptim1Asynchronous => unimplemented!(),
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
            Gpio(_, pin) => {
                use self::Pin::*;
                self.lines_used.set_bit(pin as u8, false);
                match pin {
                    Pin0 => self.exti.imr.update(|r| r.set_mr0(false)),
                    Pin1 => self.exti.imr.update(|r| r.set_mr1(false)),
                    Pin2 => self.exti.imr.update(|r| r.set_mr2(false)),
                    Pin3 => self.exti.imr.update(|r| r.set_mr3(false)),
                    Pin4 => self.exti.imr.update(|r| r.set_mr4(false)),
                    Pin5 => self.exti.imr.update(|r| r.set_mr5(false)),
                    Pin6 => self.exti.imr.update(|r| r.set_mr6(false)),
                    Pin7 => self.exti.imr.update(|r| r.set_mr7(false)),
                    Pin8 => self.exti.imr.update(|r| r.set_mr8(false)),
                    Pin9 => self.exti.imr.update(|r| r.set_mr9(false)),
                    Pin10 => self.exti.imr.update(|r| r.set_mr10(false)),
                    Pin11 => self.exti.imr.update(|r| r.set_mr11(false)),
                    Pin12 => self.exti.imr.update(|r| r.set_mr12(false)),
                    Pin13 => self.exti.imr.update(|r| r.set_mr13(false)),
                    Pin14 => self.exti.imr.update(|r| r.set_mr14(false)),
                    Pin15 => self.exti.imr.update(|r| r.set_mr15(false)),
                }
            }
            PvdOutput => {
                self.exti.imr.update(|r| r.set_mr16(false));
                self.lines_used.set_bit(16, false);
            }
            RtcAlarm => {
                self.exti.imr.update(|r| r.set_mr17(false));
                self.lines_used.set_bit(17, false);
            }
            UsbOtgFsWakeup => {
                self.exti.imr.update(|r| r.set_mr18(false));
                self.lines_used.set_bit(18, false);
            }
            EthernetWakeup => {
                self.exti.imr.update(|r| r.set_mr19(false));
                self.lines_used.set_bit(19, false);
            }
            UsbOtgHsWakeup => {
                self.exti.imr.update(|r| r.set_mr20(false));
                self.lines_used.set_bit(20, false);
            }
            RtcTamperAndTimeStamp => {
                self.exti.imr.update(|r| r.set_mr21(false));
                self.lines_used.set_bit(21, false);
            }
            RtcWakeup => {
                self.exti.imr.update(|r| r.set_mr22(false));
                self.lines_used.set_bit(22, false);
            }
            // Last line is missing in embedded_stm32f7
            Lptim1Asynchronous => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub struct LineAlreadyUsedError(ExtiLine);

pub struct ExtiHandle {
    exti_line: ExtiLine,
    pr: PrRef,
}

impl ExtiHandle {
    pub fn clear_pending_state(&mut self) {
        self.pr.set(self.exti_line, true);
    }
}

/// This enum represents the possible lines of the exti controller.
/// The `Gpio` variant is used to enable an interrupt for a GPIO-Pin. The Attributes are the `Port`
/// and the `Pin` of the used GPIO-Pin and are used to configure the MUX. A GPIO-Pin with `Pin` = x
/// is always mapped to the EXTIx line. For further information look at the reference manuel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtiLine {
    Gpio(Port, Pin),
    PvdOutput,
    RtcAlarm,
    UsbOtgFsWakeup,
    EthernetWakeup,
    UsbOtgHsWakeup,
    RtcTamperAndTimeStamp,
    RtcWakeup,
    Lptim1Asynchronous,
}

pub enum EdgeDetection {
    RisingEdge,
    FallingEdge,
    BothEdges,
}

struct PrRef(*mut ReadWrite<exti::Pr>);

unsafe impl Send for PrRef {}

impl PrRef {
    fn set(&self, exti_line: ExtiLine, value: bool) {
        use self::exti::Pr;
        let mut pr = Pr::default();

        use self::ExtiLine::*;

        match exti_line {
            Gpio(_, pin) => {
                use self::Pin::*;
                match pin {
                    Pin0 => pr.set_pr0(value),
                    Pin1 => pr.set_pr1(value),
                    Pin2 => pr.set_pr2(value),
                    Pin3 => pr.set_pr3(value),
                    Pin4 => pr.set_pr4(value),
                    Pin5 => pr.set_pr5(value),
                    Pin6 => pr.set_pr6(value),
                    Pin7 => pr.set_pr7(value),
                    Pin8 => pr.set_pr8(value),
                    Pin9 => pr.set_pr9(value),
                    Pin10 => pr.set_pr10(value),
                    Pin11 => pr.set_pr11(value),
                    Pin12 => pr.set_pr12(value),
                    Pin13 => pr.set_pr13(value),
                    Pin14 => pr.set_pr14(value),
                    Pin15 => pr.set_pr15(value),
                }
            }
            PvdOutput => pr.set_pr16(value),
            RtcAlarm => pr.set_pr17(value),
            UsbOtgFsWakeup => pr.set_pr18(value),
            EthernetWakeup => pr.set_pr19(value),
            UsbOtgHsWakeup => pr.set_pr20(value),
            RtcTamperAndTimeStamp => pr.set_pr21(value),
            RtcWakeup => pr.set_pr22(value),
            // Last line is missing in embedded_stm32f7
            Lptim1Asynchronous => unimplemented!(),
        }

        unsafe {
            (&mut *self.0).write(pr);
        };
    }
}
