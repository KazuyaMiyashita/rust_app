//! # GPIO 'Blinky' Example
//!
//! This application demonstrates how to control a GPIO pin on the RP2040.
//!
//! It may need to be adapted to your particular board layout and/or pin assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Alias for our HAL crate
use rp2040_hal as hal;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use hal::pac;
use hal::gpio::{Pins, Pin, FunctionI2C, PullUp};
use hal::I2C;
use hal::fugit::RateExtU32;
use hal::entry;

// Some traits we need
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::blocking::i2c::Write;
use rp2040_hal::i2c;

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

    // // Configure GPIO25 as an output
    // let mut led_pin = pins.gpio25.into_push_pull_output();
    // loop {
    //     led_pin.set_high().unwrap();
    //     timer.delay_ms(500);
    //     led_pin.set_low().unwrap();
    //     timer.delay_ms(500);
    // }

    let mut pico_led = pins.gpio25.into_push_pull_output();
    pico_led.set_high().unwrap();

    // AE-RO2040用
    let button_0 = pins.gpio18.into_pull_down_input();
    let button_1 = pins.gpio19.into_pull_down_input();
    let button_2 = pins.gpio20.into_pull_down_input();
    let button_3 = pins.gpio21.into_pull_down_input();

    let mut led_0 = pins.gpio14.into_push_pull_output();
    let mut led_1 = pins.gpio13.into_push_pull_output();
    let mut led_2 = pins.gpio12.into_push_pull_output();
    let mut led_3 = pins.gpio11.into_push_pull_output();

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
    let display_i2c_addr: u8 = 0x3e;
    match i2c.write(display_i2c_addr.clone(), &[0x00, 0x38, 0x39, 0x14, 0x70, 0x56, 0x6c]) {
        Ok(()) => {
            // led_0.set_high().unwrap();
            ()
        },
        Err(e) => {
            match e {
                i2c::Error::Abort(_) => led_0.set_high().unwrap(), // これだった
                i2c::Error::InvalidReadBufferLength => led_1.set_high().unwrap(),
                i2c::Error::InvalidWriteBufferLength => led_2.set_high().unwrap(),
                i2c::Error::AddressOutOfRange(_) => led_3.set_high().unwrap(),
                i2c::Error::AddressReserved(_) => led_3.set_high().unwrap(),
                _ => ()
            }
        }
    }
    timer.delay_ms(200);
    match i2c.write(display_i2c_addr.clone(), &[0x00, 0x38, 0x0d, 0x01]) {
        Ok(()) => {
            // led_2.set_high().unwrap();
            ()
        },
        Err(_) => {
            // led_3.set_high().unwrap();
            ()
        }
    }
    // https://www.junk-works.science/specification-aqm0802/

    loop {
        // if button_0.is_high().unwrap() {
        //     led_0.set_high().unwrap();
        // } else {
        //     led_0.set_low().unwrap();
        // }
        // if button_1.is_high().unwrap() {
        //     led_1.set_high().unwrap();
        // } else {
        //     led_1.set_low().unwrap();
        // }
        // if button_2.is_high().unwrap() {
        //     led_2.set_high().unwrap();
        // } else {
        //     led_2.set_low().unwrap();
        // }
        // if button_3.is_high().unwrap() {
        //     led_3.set_high().unwrap();
        // } else {
        //     led_3.set_low().unwrap();
        // }
    }
}
