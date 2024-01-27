#![no_std]
#![no_main]

use rp_pico as bsp;

use bsp::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;

use bsp::hal::{clocks::init_clocks_and_plls, pac, watchdog::Watchdog, Timer};
use fugit::MicrosDurationU64;

use cortex_m::asm::wfi;

#[entry]
fn main() -> ! {
    info!("Program start");

    let mut pac = pac::Peripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    for n in 0..50 {
        let (duration, res) = time(timer, || fib(n));
        info!("fib({}) == {}, took {} ms", n, res, duration.to_millis());
    }

    loop {
        wfi();
    }
}

fn fib(n: u128) -> u128 {
    if n <= 1 {
        1
    } else {
        fib(n - 1) + fib(n - 2)
    }
}

fn time<F, A>(timer: Timer, f: F) -> (MicrosDurationU64, A)
where
    F: Fn() -> A,
{
    let start = timer.get_counter();
    let res = f();
    let end = timer.get_counter();
    (end - start, res)
}
