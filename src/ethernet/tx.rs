use bit_field::BitField;
use core::convert::TryInto;
use core::{mem, slice};
use alloc::boxed::Box;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TxDescriptor {
    word_0: u32,
    word_1: u32,
    word_2: u32,
    word_3: u32,
}

impl TxDescriptor {
    pub const fn empty() -> TxDescriptor {
        TxDescriptor {
            word_0: 0,
            word_1: 0,
            word_2: 0,
            word_3: 0,
        }
    }

    pub fn set_end_of_ring(&mut self, value: bool) {
        self.word_0.set_bit(21, value);
    }

    pub fn set_data(&mut self, data: Box<[u8]>) {
        assert!(!self.own(), "descriptor is still owned by the hardware");

        mem::drop(self.buffer()); // drop old buffer if not already dropped

        self.set_buffer(data);
        self.set_first_segment(true);
        self.set_last_segment(true);
        self.set_own(true);
    }

    pub fn own(&self) -> bool {
        self.word_0.get_bit(31)
    }

    pub fn buffer(&mut self) -> Option<Box<[u8]>> {
        assert!(!self.own(), "descriptor is still owned by the hardware");
        match self.buffer_1_address() {
            0 => None,
            ptr => {
                self.set_buffer_1_address(0);
                Some(unsafe {
                         Box::from_raw(slice::from_raw_parts_mut(ptr as *mut u8,
                                                                 self.buffer_1_size()))
                     })
            }
        }
    }

    fn set_own(&mut self, value: bool) {
        self.word_0.set_bit(31, value);
    }

    fn set_first_segment(&mut self, value: bool) {
        self.word_0.set_bit(28, value);
    }

    fn set_last_segment(&mut self, value: bool) {
        self.word_0.set_bit(29, value);
    }

    fn set_buffer(&mut self, buffer: Box<[u8]>) {
        assert_eq!(self.buffer_1_address(), 0);
        self.set_buffer_1_address(buffer.as_ptr() as usize);
        self.set_buffer_1_size(buffer.len());
        mem::forget(buffer);
    }

    fn buffer_1_address(&self) -> usize {
        self.word_2.try_into().unwrap()
    }

    fn set_buffer_1_address(&mut self, buffer_address: usize) {
        self.word_2 = buffer_address.try_into().unwrap();
    }

    fn buffer_1_size(&self) -> usize {
        self.word_1.get_bits(0..13).try_into().unwrap()
    }

    fn set_buffer_1_size(&mut self, size: usize) {
        self.word_1
            .set_bits(0..13, size.try_into().expect("buffer too large"));
    }
}
