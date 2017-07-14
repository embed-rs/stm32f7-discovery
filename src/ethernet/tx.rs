use bit_field::BitField;
use core::convert::TryInto;

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

    pub fn set_data(&mut self, buffer_address: *const u8, buffer_len: usize) {
        assert!(!self.own(), "descriptor is still owned by the hardware");

        self.set_buffer(buffer_address, buffer_len);
        self.set_first_segment(true);
        self.set_last_segment(true);
        self.set_own(true);
    }

    pub fn own(&self) -> bool {
        self.word_0.get_bit(31)
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

    fn set_buffer(&mut self, buffer_address: *const u8, buffer_len: usize) {
        self.set_buffer_1_address(buffer_address as usize);
        self.set_buffer_1_size(buffer_len);
    }

    fn set_buffer_1_address(&mut self, buffer_address: usize) {
        self.word_2 = buffer_address.try_into().unwrap();
    }

    fn set_buffer_1_size(&mut self, size: usize) {
        self.word_1
            .set_bits(0..13, size.try_into().expect("buffer too large"));
    }
}
