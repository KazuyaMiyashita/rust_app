#![no_std]
#![no_main]

use rp_pico as bsp;

use bsp::entry;
use cortex_m_rt::exception;
use defmt::{info, error};
use defmt_rtt as _;
use panic_probe as _;

#[entry]
fn main() -> ! {
    info!("Program start");

    unsafe {
        // Supervisor Call
        core::arch::asm!("svc #0");
    }

    loop {}
}

#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    error!("IRQn = {}", irqn); // -5
}
