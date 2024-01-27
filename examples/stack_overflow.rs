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

    func(0)
}

#[allow(unconditional_recursion)]
fn func(n: u32) -> ! {

    info!("n: {}", n); // n: 10869 まで表示される
    func(n + 1)

}

// 呼ばれない
#[exception]
unsafe fn DefaultHandler(irqn: i16) {
    error!("IRQn = {}", irqn);
}
