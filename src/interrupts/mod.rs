//! Interrupts

use alloc::boxed::Box;
use board::nvic::Nvic;
use board::nvic::Stir;
use core::marker::PhantomData;
use core::intrinsics::transmute;
use self::interrupt_request::InterruptRequest;

pub mod interrupt_request;
pub mod primask_mutex;

macro_rules! create_table_and_handler {
    ($($name:ident, $irq:expr),*) => {
        /// Interrupt vector table
        #[no_mangle]
        #[used]
        #[allow(private_no_mangle_statics)]
        static INTERRUPTS: [unsafe extern "C" fn(); 98] = [$($name,)*];

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

// Unreachable at the moment (only when interrupt was enabled before InterruptTable got ownership...)
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

pub struct InterruptHandle<T> {
    _data_type: PhantomData<T>,
    irq: InterruptRequest,
}

impl<T> InterruptHandle<T> {
    fn new(irq: InterruptRequest) -> Self {
        InterruptHandle { 
            irq: irq,
            _data_type: PhantomData,
        }
    }
}

pub struct InterruptTable<'a> {
    _lifetime: PhantomData<&'a ()>,
    nvic: &'static mut Nvic,
    data: [Option<* mut ()>; 98],
}

impl<'a> Drop for InterruptTable<'a> {
    fn drop(&mut self) {
        let mut some_left = false;
        unsafe {
            DEFAULT_HANDLER = None;
            for (i,isr) in ISRS.iter().enumerate() {
                some_left = some_left || isr.is_some();
                self.dissable_interrupt(i as u8);
            }
        }
        if some_left {
            panic!("Disable interrupts first");
        }
    }
}

pub fn scope<'a,F,C,R>(nvic: &'static mut Nvic, default_handler: F, code: C) -> R 
    where F: FnMut(u8) + 'a,
        C: FnOnce(&mut InterruptTable<'a>) -> R
{
    unsafe {
        debug_assert!(DEFAULT_HANDLER.is_none());
        DEFAULT_HANDLER = Some(transmute::<Box<FnMut(u8) + 'a>, Box<FnMut(u8) + 'static>>(Box::new(default_handler)));
    }
    
    let mut interrupt_table = InterruptTable {
        _lifetime: PhantomData,
        nvic: nvic,
        data: [None; 98],
    };
    // When the *code(self)* panics, the programm ends in an endless loop with disabled interrupts
    // and never returns. So the state of the ISRS does't matter.
    code(&mut interrupt_table)

    // Drop is called
}

impl<'a> InterruptTable<'a> {

    pub fn register_static<F>(&mut self,
                              irq: InterruptRequest,
                              priority: Priority,
                              isr: F)
                              -> Result<InterruptHandle<()>, Error>
        where F: FnMut() + 'a + Send
    {
        let interrupt_handle = self.insert_boxed_isr(irq, unsafe {transmute::<Box<FnMut() + 'a + Send>, Box<FnMut() + 'static + Send>>(Box::new(isr))})?;

        self.set_priority(&interrupt_handle, priority);

        self.enable_interrupt(interrupt_handle.irq as u8);

        Ok(interrupt_handle)

    }

    pub fn with_interrupt<F, C>(&mut self,
                                       irq: InterruptRequest,
                                       priority: Priority,
                                       isr: F,
                                       code: C)
                                       -> Result<(), Error>
        where F: FnMut() + Send,
              C: FnOnce(&mut InterruptTable)
    {

        // Safe: Isr is removed from the static array after the closure *code* is executed.
        // When the *code(self)* panics, the programm ends in an endless loop with disabled interrupts
        // and never returns. So the state of the ISRS does't matter.
        let isr = unsafe {
            transmute::<Box<FnMut() + Send>,Box<FnMut() + 'static + Send>>(Box::new(isr))
        };
        let interrupt_handle = self.insert_boxed_isr::<()>(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.enable_interrupt(interrupt_handle.irq as u8);

        code(self);

        self.unregister(interrupt_handle);

        Ok(())
    }

    pub fn register<F, T>(&mut self,
                                 irq: InterruptRequest,
                                 priority: Priority,
                                 owned_data: T,
                                 mut isr: F)
                                 -> Result<InterruptHandle<T>, Error>
        where   T: Send,
                F: FnMut(&mut T) + 'a + Send
    {
        if unsafe{ISRS[irq as usize].is_some()} {
            return Err(Error::InterruptAlreadyInUse(irq));
        }
        // Insert data only, when interrupt isn't used, therefore nobody reads the data => no dataraces
        self.data[irq as usize] = unsafe {
            transmute::<Option<*mut T>, Option<*mut ()>>(Some(Box::into_raw(Box::new(owned_data))))
        };
        
        // transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>> is safe, because of the drop implementation of InterruptTable ('static is not needed for closure)
        // and alway only one isr can access the data (Send is not needed for closure)
        let isr = unsafe {
            transmute::<Box<FnMut()>, Box<FnMut() + 'static + Send>>(Box::new(
            || {
                match self.data[irq as usize] {
                    // Safe, since the correct type is known
                    Some(ptr) => isr(Box::from_raw(transmute::<*mut (), *mut T>(ptr)).as_mut()),
                    None => unreachable!("No data set"),
                }
            }))
        };
        let interrupt_handle = self.insert_boxed_isr(irq, isr)?;
        self.set_priority(&interrupt_handle, priority);
        self.enable_interrupt(interrupt_handle.irq as u8);

        Ok(interrupt_handle)
    }

    fn insert_boxed_isr<T>(&mut self,
                        irq: InterruptRequest,
                        isr_boxed: Box<FnMut() + 'static + Send>)
                        -> Result<InterruptHandle<T>, Error> {
        // Check if interrupt already in use
        if unsafe{ISRS[irq as usize].is_some()} {
            return Err(Error::InterruptAlreadyInUse(irq));
        }
        unsafe {
            ISRS[irq as usize] = Some(isr_boxed);
        }

        Ok(InterruptHandle::new(irq))
    }

    fn enable_interrupt(&mut self, irq: u8) {
        assert!(irq < 98);
        let iser_num = irq as u8 / 32u8;
        let iser_bit = irq as u8 % 32u8;

        self.nvic.iser[iser_num as usize].update(|r| {
            let old = r.setena();
            r.set_setena(old | 1 << iser_bit);
        });
    }
    
    pub fn unregister<T>(&mut self, interrupt_handle: InterruptHandle<T>) -> Option<T> {
        let irq = interrupt_handle.irq;
        self.dissable_interrupt(irq as u8);

        match self.data[irq as usize].take() {
            Some(x) => {
                    // Safe: Type T is stored in interrupt_handle
                    Some(*unsafe {Box::from_raw(transmute::<*mut (), *mut T>(x))})
                },
            None => None,
        }

    }

    fn dissable_interrupt(&mut self, irq: u8) {
        assert!(irq < 98);

        let icer_num = irq as u8 / 32u8;
        let icer_bit = irq as u8 % 32u8;

        self.nvic.icer[icer_num as usize].update(|r| {
            let old = r.clrena();
            r.set_clrena(old | 1 << icer_bit);
        });

        unsafe {
            ISRS[irq as usize] = None;
        }
    }


    // The STM32F7 only supports 16 priority levels
    // Assert that priority < 16
    pub fn set_priority<T>(&mut self, interrupt_handle: &InterruptHandle<T>, priority: Priority) {
        let irq = interrupt_handle.irq;

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used for priority.
        let priority = (priority as u8) << 4;

        self.nvic.ipr[irq as usize].update(|r| r.set(priority));
    }



    pub fn get_priority<T>(&self, interrupt_handle: &InterruptHandle<T>) -> Priority {
        let irq = interrupt_handle.irq;

        let res = self.nvic.ipr[irq as usize].read().get();

        // STM32F7 only uses 4 bits for Priority. priority << 4, because the upper 4 bits are used for priority.
        match Priority::from_u8(res >> 4) {
            Ok(priority) => priority,
            Err(PriorityDoesNotExistError(prio_number)) => {
                unreachable!("Priority {} does not exist", prio_number)
            }
        }

    }

    pub fn clear_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T>) {
        let irq = interrupt_handle.irq;
        let icpr_num = irq as u8 / 32u8;
        let icpr_bit = irq as u8 % 32u8;

        self.nvic.icpr[icpr_num as usize].update(|r| {
            let old = r.clrpend();
            r.set_clrpend(old | 1 << icpr_bit);
        });
    }

    pub fn set_pending_state<T>(&mut self, interrupt_handle: &InterruptHandle<T>) {
        let irq = interrupt_handle.irq;
        let ispr_num = irq as u8 / 32u8;
        let ispr_bit = irq as u8 % 32u8;

        self.nvic.ispr[ispr_num as usize].update(|r| {
            let old = r.setpend();
            r.set_setpend(old | 1 << ispr_bit);
        });
    }

    pub fn get_pending_state<T>(&self, interrupt_handle: &InterruptHandle<T>) -> bool {
        let irq = interrupt_handle.irq;
        let ispr_num = irq as u8 / 32u8;
        let ispr_bit = irq as u8 % 32u8;

        let reg = self.nvic.ispr[ispr_num as usize].read().setpend();
        reg & (1 << ispr_bit) != 0
    }

    pub fn trigger(&mut self, irq: InterruptRequest) {
        let mut stir = Stir::default();
        stir.set_intid(irq as u16);
        self.nvic.stir.write(stir);
    }
}

create_table_and_handler!(interrupt_handler_0,
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
struct PriorityDoesNotExistError(u8);

impl Priority {
    // use FromPrimitive?
    fn from_u8(priority: u8) -> Result<Priority, PriorityDoesNotExistError> {
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
            _ => Err(PriorityDoesNotExistError(priority)),
        }
    }
}

pub unsafe fn wfi() {
    ::cortex_m::asm::wfi();
}
