//! Interrupts

use alloc::boxed::Box;
use board::nvic::Nvic;
use board::nvic::Stir;
use self::interrupt_request::InterruptRequest;

pub mod interrupt_request;
pub mod primask_mutex;

macro_rules! set_priority_with_offset {
    ($($name:ident).* , $offset:expr , $priority:expr) => {
        match $offset {
            0 => $($name.)*update(|r| {
                r.set_ipr_n0($priority);
            }),
            1 => $($name.)*update(|r| {
                r.set_ipr_n1($priority);
            }),
            2 => $($name.)*update(|r| {
                r.set_ipr_n2($priority);
            }),
            3 => $($name.)*update(|r| {
                r.set_ipr_n3($priority);
            }),
            _ => unreachable!(),
        }
    }
}

macro_rules! get_priority_with_offset {
    ($($name:ident).* , $offset:expr) => {
        match $offset {
            0 => $($name.)*read().ipr_n0(),
            1 => $($name.)*read().ipr_n1(),
            2 => $($name.)*read().ipr_n2(),
            3 => $($name.)*read().ipr_n3(),
            _ => unreachable!(),
        }
    }
}

macro_rules! assign_interrupt_handler {
    ($( $name:ident ),*) => {
        [
            $(
                $name,
            )*
        ]
    }
}


macro_rules! create_interrupt_handler {
    ($($name:ident, $irq:expr),*) => {
        $(
            unsafe extern "C" fn $name() {
            match ISRS[$irq] {
                Some(ref mut isr) => isr(),
                None => default_handler($irq),
            }
        }
        )*
    }
}

/// Interrupt vector table
#[no_mangle]
#[used]
#[allow(private_no_mangle_statics)]
static INTERRUPTS: [unsafe extern "C" fn(); 98] = assign_interrupt_handler!(interrupt_handler_0,
                                                                            interrupt_handler_1,
                                                                            interrupt_handler_2,
                                                                            interrupt_handler_3,
                                                                            interrupt_handler_4,
                                                                            interrupt_handler_5,
                                                                            interrupt_handler_6,
                                                                            interrupt_handler_7,
                                                                            interrupt_handler_8,
                                                                            interrupt_handler_9,
                                                                            interrupt_handler_10,
                                                                            interrupt_handler_11,
                                                                            interrupt_handler_12,
                                                                            interrupt_handler_13,
                                                                            interrupt_handler_14,
                                                                            interrupt_handler_15,
                                                                            interrupt_handler_16,
                                                                            interrupt_handler_17,
                                                                            interrupt_handler_18,
                                                                            interrupt_handler_19,
                                                                            interrupt_handler_20,
                                                                            interrupt_handler_21,
                                                                            interrupt_handler_22,
                                                                            interrupt_handler_23,
                                                                            interrupt_handler_24,
                                                                            interrupt_handler_25,
                                                                            interrupt_handler_26,
                                                                            interrupt_handler_27,
                                                                            interrupt_handler_28,
                                                                            interrupt_handler_29,
                                                                            interrupt_handler_30,
                                                                            interrupt_handler_31,
                                                                            interrupt_handler_32,
                                                                            interrupt_handler_33,
                                                                            interrupt_handler_34,
                                                                            interrupt_handler_35,
                                                                            interrupt_handler_36,
                                                                            interrupt_handler_37,
                                                                            interrupt_handler_38,
                                                                            interrupt_handler_39,
                                                                            interrupt_handler_40,
                                                                            interrupt_handler_41,
                                                                            interrupt_handler_42,
                                                                            interrupt_handler_43,
                                                                            interrupt_handler_44,
                                                                            interrupt_handler_45,
                                                                            interrupt_handler_46,
                                                                            interrupt_handler_47,
                                                                            interrupt_handler_48,
                                                                            interrupt_handler_49,
                                                                            interrupt_handler_50,
                                                                            interrupt_handler_51,
                                                                            interrupt_handler_52,
                                                                            interrupt_handler_53,
                                                                            interrupt_handler_54,
                                                                            interrupt_handler_55,
                                                                            interrupt_handler_56,
                                                                            interrupt_handler_57,
                                                                            interrupt_handler_58,
                                                                            interrupt_handler_59,
                                                                            interrupt_handler_60,
                                                                            interrupt_handler_61,
                                                                            interrupt_handler_62,
                                                                            interrupt_handler_63,
                                                                            interrupt_handler_64,
                                                                            interrupt_handler_65,
                                                                            interrupt_handler_66,
                                                                            interrupt_handler_67,
                                                                            interrupt_handler_68,
                                                                            interrupt_handler_69,
                                                                            interrupt_handler_70,
                                                                            interrupt_handler_71,
                                                                            interrupt_handler_72,
                                                                            interrupt_handler_73,
                                                                            interrupt_handler_74,
                                                                            interrupt_handler_75,
                                                                            interrupt_handler_76,
                                                                            interrupt_handler_77,
                                                                            interrupt_handler_78,
                                                                            interrupt_handler_79,
                                                                            interrupt_handler_80,
                                                                            interrupt_handler_81,
                                                                            interrupt_handler_82,
                                                                            interrupt_handler_83,
                                                                            interrupt_handler_84,
                                                                            interrupt_handler_85,
                                                                            interrupt_handler_86,
                                                                            interrupt_handler_87,
                                                                            interrupt_handler_88,
                                                                            interrupt_handler_89,
                                                                            interrupt_handler_90,
                                                                            interrupt_handler_91,
                                                                            interrupt_handler_92,
                                                                            interrupt_handler_93,
                                                                            interrupt_handler_94,
                                                                            interrupt_handler_95,
                                                                            interrupt_handler_96,
                                                                            interrupt_handler_97);




static mut ISRS: [Option<Box<FnMut()>>; 98] =
    [None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
     None, None, None, None, None, None, None, None];

/// Default interrupt handler
static mut DEFAULT_HANDLER: Option<Box<FnMut(u8)>> = None;

fn default_handler(irq: u8) {
    unsafe {
        match DEFAULT_HANDLER {
            Some(ref mut handler) => handler(irq),
            None => panic!("No default handler"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InterruptAlreadyInUse(InterruptRequest),
}

pub struct InterruptHandle {
    irq: InterruptRequest,
}

impl InterruptHandle {
    fn new(irq: InterruptRequest) -> Self {
        InterruptHandle { irq: irq }
    }
}

pub struct InterruptHandler {
    nvic: &'static mut Nvic,
    used_interrupts: [bool; 98],
}

impl InterruptHandler {
    pub fn new<F>(nvic: &'static mut Nvic, default_handler: F) -> Self
        where F: FnMut(u8) + 'static
    {
        unsafe {
            DEFAULT_HANDLER = Some(Box::new(default_handler));
        }
        InterruptHandler {
            nvic: nvic,
            used_interrupts: [false; 98],
        }
    }

    pub fn register_isr<F>(&mut self,
                           irq: InterruptRequest,
                           priority: Priority,
                           isr: F)
                           -> Result<InterruptHandle, Error>
        where F: FnMut() + 'static + Send
    {
        // Check if interrupt already in use
        if self.used_interrupts[irq as usize] {
            return Err(Error::InterruptAlreadyInUse(irq));
        }
        self.used_interrupts[irq as usize] = true;
        unsafe {
            ISRS[irq as usize] = Some(Box::new(isr));
        }

        let iser_mun = irq as u8 / 32u8;
        let iser_bit = irq as u8 % 32u8;

        let interrupt_handle = InterruptHandle::new(irq);
        self.set_priority(&interrupt_handle, priority);

        self.enable_interrupt(iser_mun, iser_bit);

        Ok(interrupt_handle)

    }

    fn enable_interrupt(&mut self, iser_num: u8, iser_bit: u8) {
        match iser_num {
            0 => {
                self.nvic
                    .iser0
                    .update(|r| {
                                let old = r.setena();
                                r.set_setena(old | 1 << iser_bit);
                            })
            }
            1 => {
                self.nvic
                    .iser1
                    .update(|r| {
                                let old = r.setena();
                                r.set_setena(old | 1 << iser_bit);
                            })
            }
            2 => {
                self.nvic
                    .iser2
                    .update(|r| {
                                let old = r.setena();
                                r.set_setena(old | 1 << iser_bit);
                            })
            }
            // iser3 missing? 98 div 32 = 3
            /*3 => {
                self.nvic
                    .iser3
                    .update(|r| {
                                let old = r.setena();
                                r.set_setena(old | 1 << iser_bit);
                            })
            }*/
            _ => unreachable!(),
        }
    }

    pub fn unregister_isr(&mut self, interrupt_handle: InterruptHandle) {
        let irq = interrupt_handle.irq;
        let icer_num = irq as u8 / 32u8;
        let icer_bit = irq as u8 % 32u8;

        match icer_num {
            0 => {
                self.nvic
                    .icer0
                    .update(|r| {
                                let old = r.clrena();
                                r.set_clrena(old | 1 << icer_bit);
                            })
            }
            1 => {
                self.nvic
                    .icer1
                    .update(|r| {
                                let old = r.clrena();
                                r.set_clrena(old | 1 << icer_bit);
                            })
            }
            2 => {
                self.nvic
                    .icer2
                    .update(|r| {
                                let old = r.clrena();
                                r.set_clrena(old | 1 << icer_bit);
                            })
            }
            // icer3 missing? ... 97 div 32 = 3
            /*3 => self.nvic.icer3.update(|r| {
                let old = r.clrena();
                r.set_clrena(old | 1 << icer_num);
            }),*/
            _ => unreachable!(),
        }

        unsafe {
            ISRS[irq as usize] = None;
        }

        self.used_interrupts[irq as usize] = false;

    }



    // The STM32F7 only supports 16 priority levels
    // Assert that priority < 16
    pub fn set_priority(&mut self, interrupt_handle: &InterruptHandle, priority: Priority) {
        let irq = interrupt_handle.irq;
        let ipr_num = irq as u8 / 4u8;
        let ipr_offset = irq as u8 % 4u8;

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used for priority.
        let priority = (priority as u8) << 4;

        match ipr_num {
            0 => set_priority_with_offset!(self.nvic.ipr0, ipr_offset, priority),
            2 => set_priority_with_offset!(self.nvic.ipr1, ipr_offset, priority),
            1 => set_priority_with_offset!(self.nvic.ipr2, ipr_offset, priority), 
            3 => set_priority_with_offset!(self.nvic.ipr3, ipr_offset, priority), 
            4 => set_priority_with_offset!(self.nvic.ipr4, ipr_offset, priority), 
            5 => set_priority_with_offset!(self.nvic.ipr5, ipr_offset, priority), 
            6 => set_priority_with_offset!(self.nvic.ipr6, ipr_offset, priority), 
            7 => set_priority_with_offset!(self.nvic.ipr7, ipr_offset, priority), 
            8 => set_priority_with_offset!(self.nvic.ipr8, ipr_offset, priority), 
            9 => set_priority_with_offset!(self.nvic.ipr9, ipr_offset, priority), 
            10 => set_priority_with_offset!(self.nvic.ipr10, ipr_offset, priority),
            11 => set_priority_with_offset!(self.nvic.ipr11, ipr_offset, priority),
            12 => set_priority_with_offset!(self.nvic.ipr12, ipr_offset, priority),
            13 => set_priority_with_offset!(self.nvic.ipr13, ipr_offset, priority),
            14 => set_priority_with_offset!(self.nvic.ipr14, ipr_offset, priority),
            15 => set_priority_with_offset!(self.nvic.ipr15, ipr_offset, priority),
            16 => set_priority_with_offset!(self.nvic.ipr16, ipr_offset, priority),
            17 => set_priority_with_offset!(self.nvic.ipr17, ipr_offset, priority),
            18 => set_priority_with_offset!(self.nvic.ipr18, ipr_offset, priority),
            19 => set_priority_with_offset!(self.nvic.ipr19, ipr_offset, priority),
            20 => set_priority_with_offset!(self.nvic.ipr20, ipr_offset, priority),
            // 21,22,23,24 missing? 97 div 4 = 24
            _ => unreachable!(),
        }
    }



    pub fn get_priority(&self, interrupt_handle: &InterruptHandle) -> Priority {
        let irq = interrupt_handle.irq;
        let ipr_num = irq as u8 / 4u8;
        let ipr_offset = irq as u8 % 4u8;

        let res = match ipr_num {
            0 => get_priority_with_offset!(self.nvic.ipr0, ipr_offset),
            2 => get_priority_with_offset!(self.nvic.ipr1, ipr_offset),
            1 => get_priority_with_offset!(self.nvic.ipr2, ipr_offset), 
            3 => get_priority_with_offset!(self.nvic.ipr3, ipr_offset), 
            4 => get_priority_with_offset!(self.nvic.ipr4, ipr_offset), 
            5 => get_priority_with_offset!(self.nvic.ipr5, ipr_offset), 
            6 => get_priority_with_offset!(self.nvic.ipr6, ipr_offset), 
            7 => get_priority_with_offset!(self.nvic.ipr7, ipr_offset), 
            8 => get_priority_with_offset!(self.nvic.ipr8, ipr_offset), 
            9 => get_priority_with_offset!(self.nvic.ipr9, ipr_offset), 
            10 => get_priority_with_offset!(self.nvic.ipr10, ipr_offset),
            11 => get_priority_with_offset!(self.nvic.ipr11, ipr_offset),
            12 => get_priority_with_offset!(self.nvic.ipr12, ipr_offset),
            13 => get_priority_with_offset!(self.nvic.ipr13, ipr_offset),
            14 => get_priority_with_offset!(self.nvic.ipr14, ipr_offset),
            15 => get_priority_with_offset!(self.nvic.ipr15, ipr_offset),
            16 => get_priority_with_offset!(self.nvic.ipr16, ipr_offset),
            17 => get_priority_with_offset!(self.nvic.ipr17, ipr_offset),
            18 => get_priority_with_offset!(self.nvic.ipr18, ipr_offset),
            19 => get_priority_with_offset!(self.nvic.ipr19, ipr_offset),
            20 => get_priority_with_offset!(self.nvic.ipr20, ipr_offset),
            // 21,22,23,24 missing? 97 div 4 = 24
            _ => unreachable!(),
        };

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used for priority.
        match Priority::from_u8(res >> 4) {
            Ok(priority) => priority,
            Err(PriorityDoesNotExitError(prio_number)) => {
                unreachable!("Priority {} does not exist", prio_number)
            }
        }

    }

    pub fn clear_pending_state(&mut self, interrupt_handle: &InterruptHandle) {
        let irq = interrupt_handle.irq;
        let icpr_num = irq as u8 / 32u8;
        let icpr_bit = irq as u8 % 32u8;

        match icpr_num {
            0 => {
                self.nvic
                    .icpr0
                    .update(|r| {
                                let old = r.clrpend();
                                r.set_clrpend(old | 1 << icpr_bit);
                            })
            }
            1 => {
                self.nvic
                    .icpr1
                    .update(|r| {
                                let old = r.clrpend();
                                r.set_clrpend(old | 1 << icpr_bit);
                            })
            }
            2 => {
                self.nvic
                    .icpr2
                    .update(|r| {
                                let old = r.clrpend();
                                r.set_clrpend(old | 1 << icpr_bit);
                            })
            }
            // icpr3 missing?
            /*3 => self.nvic.icpr3.update(|r| {
                let old = r.clrpend();
                r.set_clrpend(old | 1 << icer_num);
            }),*/
            _ => unreachable!(),
        }
    }

    pub fn set_pending_state(&mut self, interrupt_handle: &InterruptHandle) {
        let irq = interrupt_handle.irq;
        let ispr_num = irq as u8 / 32u8;
        let ispr_bit = irq as u8 % 32u8;

        match ispr_num {
            0 => {
                self.nvic
                    .ispr0
                    .update(|r| {
                                let old = r.setpend();
                                r.set_setpend(old | 1 << ispr_bit);
                            })
            }
            1 => {
                self.nvic
                    .ispr1
                    .update(|r| {
                                let old = r.setpend();
                                r.set_setpend(old | 1 << ispr_bit);
                            })
            }
            2 => {
                self.nvic
                    .ispr2
                    .update(|r| {
                                let old = r.setpend();
                                r.set_setpend(old | 1 << ispr_bit);
                            })
            }
            // ispr3 missing?
            /*3 => self.nvic.ispr3.update(|r| {
                let old = r.setpend();
                r.set_setpend(old | 1 << icer_num);
            }),*/
            _ => unreachable!(),
        }
    }

    pub fn get_pending_state(&self, interrupt_handle: &InterruptHandle) -> bool {
        let irq = interrupt_handle.irq;
        let ispr_num = irq as u8 / 32u8;
        let ispr_bit = irq as u8 % 32u8;

        let reg = match ispr_num {
            0 => self.nvic.ispr0.read().setpend(),
            1 => self.nvic.ispr1.read().setpend(),
            2 => self.nvic.ispr2.read().setpend(),
            // ispr3 missing?
            //3 => self.nvic.ispr3.read.setpend(),
            _ => unreachable!(),
        };

        reg & (1 << ispr_bit) != 0
    }

    pub fn tigger(&mut self, irq: InterruptRequest) {
        let mut stir = Stir::default();
        stir.set_intid(irq as u16);
        self.nvic.stir.write(stir);
    }
}

create_interrupt_handler!(interrupt_handler_0,
                          0,
                          interrupt_handler_1,
                          1,
                          interrupt_handler_2,
                          2,
                          interrupt_handler_3,
                          3,
                          interrupt_handler_4,
                          4,
                          interrupt_handler_5,
                          5,
                          interrupt_handler_6,
                          6,
                          interrupt_handler_7,
                          7,
                          interrupt_handler_8,
                          8,
                          interrupt_handler_9,
                          9,
                          interrupt_handler_10,
                          10,
                          interrupt_handler_11,
                          11,
                          interrupt_handler_12,
                          12,
                          interrupt_handler_13,
                          13,
                          interrupt_handler_14,
                          14,
                          interrupt_handler_15,
                          15,
                          interrupt_handler_16,
                          16,
                          interrupt_handler_17,
                          17,
                          interrupt_handler_18,
                          18,
                          interrupt_handler_19,
                          19,
                          interrupt_handler_20,
                          20,
                          interrupt_handler_21,
                          21,
                          interrupt_handler_22,
                          22,
                          interrupt_handler_23,
                          23,
                          interrupt_handler_24,
                          24,
                          interrupt_handler_25,
                          25,
                          interrupt_handler_26,
                          26,
                          interrupt_handler_27,
                          27,
                          interrupt_handler_28,
                          28,
                          interrupt_handler_29,
                          29,
                          interrupt_handler_30,
                          30,
                          interrupt_handler_31,
                          31,
                          interrupt_handler_32,
                          32,
                          interrupt_handler_33,
                          33,
                          interrupt_handler_34,
                          34,
                          interrupt_handler_35,
                          35,
                          interrupt_handler_36,
                          36,
                          interrupt_handler_37,
                          37,
                          interrupt_handler_38,
                          38,
                          interrupt_handler_39,
                          39,
                          interrupt_handler_40,
                          40,
                          interrupt_handler_41,
                          41,
                          interrupt_handler_42,
                          42,
                          interrupt_handler_43,
                          43,
                          interrupt_handler_44,
                          44,
                          interrupt_handler_45,
                          45,
                          interrupt_handler_46,
                          46,
                          interrupt_handler_47,
                          47,
                          interrupt_handler_48,
                          48,
                          interrupt_handler_49,
                          49,
                          interrupt_handler_50,
                          50,
                          interrupt_handler_51,
                          51,
                          interrupt_handler_52,
                          52,
                          interrupt_handler_53,
                          53,
                          interrupt_handler_54,
                          54,
                          interrupt_handler_55,
                          55,
                          interrupt_handler_56,
                          56,
                          interrupt_handler_57,
                          57,
                          interrupt_handler_58,
                          58,
                          interrupt_handler_59,
                          59,
                          interrupt_handler_60,
                          60,
                          interrupt_handler_61,
                          61,
                          interrupt_handler_62,
                          62,
                          interrupt_handler_63,
                          63,
                          interrupt_handler_64,
                          64,
                          interrupt_handler_65,
                          65,
                          interrupt_handler_66,
                          66,
                          interrupt_handler_67,
                          67,
                          interrupt_handler_68,
                          68,
                          interrupt_handler_69,
                          69,
                          interrupt_handler_70,
                          70,
                          interrupt_handler_71,
                          71,
                          interrupt_handler_72,
                          72,
                          interrupt_handler_73,
                          73,
                          interrupt_handler_74,
                          74,
                          interrupt_handler_75,
                          75,
                          interrupt_handler_76,
                          76,
                          interrupt_handler_77,
                          77,
                          interrupt_handler_78,
                          78,
                          interrupt_handler_79,
                          79,
                          interrupt_handler_80,
                          80,
                          interrupt_handler_81,
                          81,
                          interrupt_handler_82,
                          82,
                          interrupt_handler_83,
                          83,
                          interrupt_handler_84,
                          84,
                          interrupt_handler_85,
                          85,
                          interrupt_handler_86,
                          86,
                          interrupt_handler_87,
                          87,
                          interrupt_handler_88,
                          88,
                          interrupt_handler_89,
                          89,
                          interrupt_handler_90,
                          90,
                          interrupt_handler_91,
                          91,
                          interrupt_handler_92,
                          92,
                          interrupt_handler_93,
                          93,
                          interrupt_handler_94,
                          94,
                          interrupt_handler_95,
                          95,
                          interrupt_handler_96,
                          96,
                          interrupt_handler_97,
                          97);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    P0 = 0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P9,
    P10,
    P11,
    P12,
    P13,
    P14,
    P15,
}
struct PriorityDoesNotExitError(u8);

impl Priority {
    // use FromPrimitive?
    fn from_u8(priority: u8) -> Result<Priority, PriorityDoesNotExitError> {
        use self::Priority::*;
        match priority {
            0 => Ok(P0),
            1 => Ok(P1),
            2 => Ok(P2),
            3 => Ok(P3),
            4 => Ok(P4),
            5 => Ok(P5),
            6 => Ok(P6),
            7 => Ok(P7),
            8 => Ok(P8),
            9 => Ok(P9),
            10 => Ok(P10),
            11 => Ok(P11),
            12 => Ok(P12),
            13 => Ok(P13),
            14 => Ok(P14),
            15 => Ok(P15),
            _ => Err(PriorityDoesNotExitError(priority)),
        }
    }
}

pub unsafe fn wfi() {
    ::cortex_m::asm::wfi();
}
