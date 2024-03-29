//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod button_input_queue;
mod console;
mod display_aqm0802;
mod global_led_pins;

use bsp::entry;
use defmt::{error, info};
use defmt_rtt as _;
use panic_probe as _;

use rp_pico as bsp;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::OutputPin;

use bsp::hal::fugit::ExtU32;
use bsp::hal::fugit::RateExtU32;
use bsp::hal::{clocks::init_clocks_and_plls, gpio, pac, sio::Sio, watchdog::Watchdog, Timer, I2C};

use crate::console::Console;
use core::fmt::Write;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;

extern crate alloc;

use crate::button_input_queue::ButtonInput;
use button_input_queue::ButtonInputQueue;
use global_led_pins::LedMode;

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

    // Scheduler::init(timer, timer.alarm_1().unwrap());

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

    ButtonInputQueue::init(
        pins.gpio19.reconfigure(),
        pins.gpio18.reconfigure(),
        pins.gpio17.reconfigure(),
        pins.gpio16.reconfigure(),
        timer,
        timer.alarm_0().unwrap(),
    );

    global_led_pins::init(
        pins.gpio13.into_push_pull_output(),
        pins.gpio12.into_push_pull_output(),
        pins.gpio11.into_push_pull_output(),
        pins.gpio10.into_push_pull_output(),
        timer,
        timer.alarm_1().unwrap(),
    );

    info!("into loop...");
    writeln!(console, "Hello!").unwrap();

    for i in 0..=3 {
        global_led_pins::set_led_mode(i, LedMode::HIGH);
        global_led_pins::set_mode_later(i, LedMode::LOW, 500.millis());
    }
    for _ in 0..=7 {
        write!(console, ".").unwrap();
        timer.delay_ms(100);
    }
    console.clear().unwrap();

    let mut status = [false; 4];

    loop {
        let pushed_buttons = ButtonInputQueue::pop_all();
        if pushed_buttons.contains(&ButtonInput::Button0) {
            if status[0] {
                writeln!(console, "Stop B0").unwrap();
                global_led_pins::set_led_mode(0, LedMode::LOW);
            } else {
                writeln!(console, "Start B0").unwrap();
                global_led_pins::set_led_mode(0, LedMode::BLINK);
            }
            status[0] = !status[0];
        } else if pushed_buttons.contains(&ButtonInput::Button1) {
            if status[1] {
                writeln!(console, "Stop B1").unwrap();
                global_led_pins::set_led_mode(1, LedMode::LOW);
            } else {
                writeln!(console, "Start B1").unwrap();
                global_led_pins::set_led_mode(1, LedMode::BLINK);
            }
            status[1] = !status[1];
        } else if pushed_buttons.contains(&ButtonInput::Button2) {
            if status[2] {
                writeln!(console, "Stop B2").unwrap();
                global_led_pins::set_led_mode(2, LedMode::LOW);
            } else {
                writeln!(console, "Start B2").unwrap();
                global_led_pins::set_led_mode(2, LedMode::BLINK);
            }
            status[2] = !status[2];
        } else if pushed_buttons.contains(&ButtonInput::Button3) {
            if status[3] {
                writeln!(console, "Stop B3").unwrap();
                global_led_pins::set_led_mode(3, LedMode::LOW);
            } else {
                writeln!(console, "Start B3").unwrap();
                global_led_pins::set_led_mode(3, LedMode::BLINK);
            }
            status[3] = !status[3];
        }

        timer.delay_ms(10);
        // interrupts handle everything else in this example.
        // cortex_m::asm::wfi();
    }
}

// End of file
