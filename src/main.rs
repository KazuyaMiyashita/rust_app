//! # GPIO 'Blinky' Example
//!
//! This application demonstrates how to control a GPIO pin on the RP2040.
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

mod display_aqm0802;
mod leds;

use defmt::info;
use defmt_rtt as _;
use panic_probe as _;

use rp2040_hal as hal;
use hal::pac;
use hal::gpio::{Pins, Pin, FunctionI2C, PullUp};
use hal::I2C;
use hal::fugit::RateExtU32;
use hal::entry;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::blocking::delay::DelayMs;

use crate::display_aqm0802::DisplayAQM0802;
use crate::leds::LEDs;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
/// Note: This boot block is not necessary when using a rp-hal based BSP
/// as the BSPs already perform this step.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// External high-speed crystal on the Raspberry Pi Pico board is 12 MHz. Adjust
/// if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Entry point to our bare-metal application.
///
/// The `#[rp2040_hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the RP2040 peripherals, then toggles a GPIO pin in
/// an infinite loop. If there is an LED connected to that pin, it will blink.
#[entry]
fn main() -> ! {
    info!("hello!");

    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).ok().unwrap();

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut pico_led = pins.gpio25.into_push_pull_output();
    pico_led.set_high().unwrap();

    let mut leds = LEDs::new(
        pins.gpio10.into_push_pull_output().into_dyn_pin(),
        pins.gpio11.into_push_pull_output().into_dyn_pin(),
        pins.gpio12.into_push_pull_output().into_dyn_pin(),
        pins.gpio13.into_push_pull_output().into_dyn_pin(),
    );
    leds.light(0).unwrap();

    // Configure two pins as being I²C, not GPIO
    let sda_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio16.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio17.reconfigure();
    // Create the I²C drive, using the two pre-configured pins. This will fail
    // at compile time if the pins are in the wrong mode, or if this I²C
    // peripheral isn't available on these pins!
    let mut i2c = I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // use embedded_hal::blocking::i2c::Write;
    // let bytes: [u8; 3] = [0x01, 0x02, 0x03];
    // i2c.write(0x2c, &bytes).unwrap();
    //
    // use embedded_hal::blocking::i2c::Read;
    // let mut readbuf: [u8; 1] = [0; 1];
    // for addr in 0..=127 {
    //     info!("{}:", addr);
    //     match i2c.read(addr, &mut readbuf) {
    //         Ok(_) => {
    //             info!("ok");
    //         }
    //         Err(e) => {
    //             match e {
    //                 hal::i2c::Error::Abort(i) => {
    //                     info!("ng. error: Abort({})", i);
    //                 }
    //                 hal::i2c::Error::InvalidReadBufferLength => {
    //                     info!("ng. error: InvalidReadBufferLength");
    //                 }
    //                 hal::i2c::Error::InvalidWriteBufferLength => {
    //                     info!("ng. error: InvalidWriteBufferLength");
    //                 }
    //                 hal::i2c::Error::AddressOutOfRange(i) => {
    //                     info!("ng. error: AddressOutOfRange({})", i);
    //                 }
    //                 hal::i2c::Error::AddressReserved(i) => {
    //                     info!("ng. error: AddressReserved({})", i);
    //                 }
    //                 _ => {
    //                     info!("ng. error: other");
    //                 }
    //             }
    //         }
    //     }
    //     timer.delay_ms(10);
    // }
    //
    // info!("???");

    let mut display = DisplayAQM0802::init_blocking(i2c, &mut timer).unwrap();

    display.print_blocking("hello").unwrap();

    info!("into loop...");

    loop {}
}
