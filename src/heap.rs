use ALLOCATOR;

extern "C" {
    static __HEAP_START: usize;
    static __HEAP_END: usize;
}

// Initialize the heap
pub unsafe fn init() {
    let start = &__HEAP_START as *const _ as usize;
    let end = &__HEAP_END as *const _ as usize;
    let size = end - start;
    ALLOCATOR.init(start, size);
}
