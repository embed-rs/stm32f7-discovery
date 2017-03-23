use alloc_cortex_m;

extern "C" {
    static mut __HEAP_START: usize;
    static mut __HEAP_END: usize;
}

// Initialize the heap
pub unsafe fn init() {
    alloc_cortex_m::init(&mut __HEAP_START, &mut __HEAP_END);
}
