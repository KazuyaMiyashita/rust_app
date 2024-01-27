#![no_std]
#![no_main]

use rp_pico as bsp;

use bsp::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use bsp::hal::{clocks::init_clocks_and_plls, pac, sio::Sio, watchdog::Watchdog, Timer};
use bsp::Pins;

#[entry]
fn main() -> ! {

    info!("Program start");

    let mut pac = pac::Peripherals::take().unwrap();
    let sio = Sio::new(pac.SIO);
    
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
    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led = pins.led.into_push_pull_output();

    loop {
        led.set_high().unwrap();
        timer.delay_ms(500);

        led.set_low().unwrap();
        timer.delay_ms(500);
    }
}
