use bit_field::BitField;
use alloc::boxed::Box;
use core::convert::TryInto;
use core::mem;

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

    pub fn new(buffer: Box<[u8]>) -> TxDescriptor {
        let mut descriptor = TxDescriptor::empty();
        descriptor.set_buffer_1(buffer);

        descriptor
    }

    fn set_next(&mut self, next: Box<TxDescriptor>) {
        self.word_3 = (Box::into_raw(next) as usize).try_into().unwrap();
    }

    pub fn own(&self) -> bool {
        self.word_0.get_bit(31)
    }

    fn set_buffer_1(&mut self, buffer: Box<[u8]>) {
        assert_eq!(self.buffer_1_address(), 0);
        self.set_buffer_1_address(buffer.as_ptr() as usize);
        self.set_buffer_1_size(buffer.len() as u16);
        mem::forget(buffer);
    }

    fn buffer_1_address(&self) -> usize {
        self.word_2.try_into().unwrap()
    }

    fn set_buffer_1_address(&mut self, buffer_address: usize) {
        self.word_2 = buffer_address.try_into().unwrap();
    }

    fn set_buffer_1_size(&mut self, size: u16) {
        self.word_1.set_bits(0..13, size.into());
    }
}
