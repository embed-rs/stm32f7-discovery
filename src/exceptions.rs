use system_clock;

#[no_mangle]
pub static EXCEPTIONS: VectorTable = VectorTable {
    nmi: None,
    hard_fault: Some(fault),
    mem_manage: None,
    bus_fault: None,
    usage_fault: None,
    svcall: None,
    debug_monitor: None,
    pendsv: None,
    sys_tick: Some(system_clock::systick),
    reserved_0: [0; 4],
    reserved_1: 0,
};

#[repr(C)]
pub struct VectorTable {
    /// Non Maskable Interrupt
    pub nmi: Option<Handler>,
    /// Hard Fault
    pub hard_fault: Option<Handler>,
    /// Memory Management
    pub mem_manage: Option<Handler>,
    /// Bus Fault
    pub bus_fault: Option<Handler>,
    /// Usage Fault
    pub usage_fault: Option<Handler>,
    reserved_0: [u32; 4],
    /// Supervisor Call
    pub svcall: Option<Handler>,
    /// Debug Monitor
    pub debug_monitor: Option<Handler>,
    reserved_1: u32,
    /// PendSV
    pub pendsv: Option<Handler>,
    /// SysTick
    pub sys_tick: Option<Handler>,
}

type Handler = extern "C" fn();

extern "C" fn fault() {
    panic!("EXCEPTION: hard fault");
}
