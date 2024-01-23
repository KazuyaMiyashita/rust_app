//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod button_interrupt;
mod console;
mod display_aqm0802;

use bsp::entry;
use defmt::{debug, error, info};
use defmt_rtt as _;
use panic_probe as _;

use rp_pico as bsp;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{InputPin, OutputPin};

use bsp::hal::fugit::RateExtU32;
use bsp::hal::{
    clocks::init_clocks_and_plls, gpio, gpio::Interrupt::EdgeHigh, pac, sio::Sio,
    watchdog::Watchdog, Timer, I2C,
};

use fugit::ExtU32;

use crate::console::Console;
use core::fmt::Write;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use rp_pico::hal::timer::Alarm;

extern crate alloc;

use button_interrupt::ButtonInterrupt;

// Pin types quickly become very long!
// We'll create some type aliases using `type` to help with that

/// This pin will be our output - it will drive an LED if you run this on a Pico
// type LedPin = gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSioOutput, gpio::PullNone>;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[alloc_error_handler]
fn oom(_: Layout) -> ! {
    error!("oom error!");
    loop {}
}

#[entry]
fn main() -> ! {
    info!("Program start");

    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024 * 20; //20KBの領域
    static mut HEAP: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { ALLOCATOR.init(HEAP.as_ptr() as usize, HEAP_SIZE) }

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

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
    let mut alarm = timer.alarm_0().unwrap();
    alarm.enable_interrupt();

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led = pins.led.into_push_pull_output();
    led.set_high().unwrap();

    // Configure two pins as being I²C, not GPIO
    let sda_pin: gpio::Pin<_, gpio::FunctionI2C, gpio::PullUp> = pins.gpio20.reconfigure();
    let scl_pin: gpio::Pin<_, gpio::FunctionI2C, gpio::PullUp> = pins.gpio21.reconfigure();
    // Create the I²C drive, using the two pre-configured pins. This will fail
    // at compile time if the pins are in the wrong mode, or if this I²C
    // peripheral isn't available on these pins!
    let i2c = I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let mut console = Console::init_blocking(i2c, &mut timer).unwrap();

    let button_interrupt = ButtonInterrupt::init(
        pins.gpio19.reconfigure(),
        pins.gpio18.reconfigure(),
        pins.gpio17.reconfigure(),
        pins.gpio16.reconfigure(),
    );

    info!("into loop...");
    writeln!(console, "Hello!").unwrap();

    for _ in 0..=7 {
        write!(console, ".").unwrap();
        timer.delay_ms(250);
    }
    writeln!(console).unwrap();

    let mut counter = 0;
    loop {
        let mut button_pressed = false;
        let _ = button_interrupt.pop();

        if button_pressed {
            counter += 1;
            writeln!(console, "c:{}", counter).unwrap();
        }

        timer.delay_ms(10);
        // interrupts handle everything else in this example.
        // cortex_m::asm::wfi();
    }
}

// End of file
