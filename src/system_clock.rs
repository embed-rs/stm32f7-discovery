use core::sync::atomic::{AtomicUsize, Ordering};

static TICKS: AtomicUsize = AtomicUsize::new(0);

pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub fn wait(ticks: usize) {
    let current = TICKS.load(Ordering::Acquire);
    let desired = current + ticks;
    while TICKS.load(Ordering::Acquire) != desired {}
}
