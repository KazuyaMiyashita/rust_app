use core::arch::x86_64::_rdtsc;

pub struct Timer {}
impl Timer {

    pub fn init() -> Timer { Timer{} }

    pub fn get_elapsed_time_ms(&self) -> u64 {
        let timestamp: u64 = unsafe { _rdtsc() };
        timestamp / 1_000_000 // ???
    }

}