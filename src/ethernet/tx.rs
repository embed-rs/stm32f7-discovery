use bit_field::BitField;
use arrayvec::ArrayVec;
use spin::Mutex;
use volatile::Volatile;

const NUMBER_OF_DESCRIPTORS: usize = 16;

pub fn init_descriptors() {
    let mut descriptors = ArrayVec::<[_; NUMBER_OF_DESCRIPTORS]>::new();
    let tx_buffers = TX_BUFFERS.lock();

    for buffer in tx_buffers.iter() {
        let descriptor = TxDescriptor::new(buffer);
        descriptors.push(descriptor);
    }

    // chain descriptors
    {
        let mut iter = descriptors.iter_mut().peekable();
        while let Some(descriptor) = iter.next() {
            if let Some(next) = iter.peek() {
                descriptor.set_next(next);
            }
        }
    }

    let descriptors: ArrayVec<[_; NUMBER_OF_DESCRIPTORS]> =
        descriptors.into_iter().map(|d| Volatile::new(d)).collect();
}


struct TxBuffer([u8; 0x100]);

impl Clone for TxBuffer {
    fn clone(&self) -> TxBuffer {
        TxBuffer(self.0)
    }
}

impl Copy for TxBuffer {}

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

    pub fn new(buffer: &[u8]) -> TxDescriptor {
        let mut descriptor = TxDescriptor::empty();
        descriptor.set_buffer_1(buffer);

        descriptor
    }

    fn set_next(&mut self, next: &TxDescriptor) {
        self.word_3 = next as *const _ as u32;
    }

    pub fn own(&self) -> bool {
        self.word_0.get_bit(31)
    }

    fn set_buffer_1(&mut self, buffer: &[u8]) {
        self.set_buffer_1_address(buffer.as_ptr() as usize);
        self.set_buffer_1_size(buffer.len() as u16);
    }

    fn set_buffer_1_address(&mut self, buffer_address: usize) {
        self.word_2 = buffer_address as u32;
    }

    fn set_buffer_1_size(&mut self, size: u16) {
        self.word_1.set_range(0..13, size.into());
    }
}
