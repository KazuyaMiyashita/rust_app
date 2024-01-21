//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

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

use bsp::hal::pac::interrupt;

use fugit::ExtU32;

use core::cell::RefCell;
use critical_section::Mutex;

use crate::console::Console;
use core::fmt::Write;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use rp_pico::hal::timer::{Alarm, Alarm0};

extern crate alloc;

// Pin types quickly become very long!
// We'll create some type aliases using `type` to help with that

/// This pin will be our output - it will drive an LED if you run this on a Pico
// type LedPin = gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSioOutput, gpio::PullNone>;

/// This pin will be our interrupt source.
/// ~~It will trigger an interrupt if pulled to ground (via a switch or jumper wire)~~
/// 電流が流れている時にONとしたいので、PullUp から PullDown に変更した
type Button0Pin = gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSioInput, gpio::PullDown>;
type Button1Pin = gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSioInput, gpio::PullDown>;
type Button2Pin = gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSioInput, gpio::PullDown>;
type Button3Pin = gpio::Pin<gpio::bank0::Gpio16, gpio::FunctionSioInput, gpio::PullDown>;

/// Since we're always accessing these pins together we'll store them in a tuple.
/// Giving this tuple a type alias means we won't need to use () when putting them
/// inside an Option. That will be easier to read.
struct KeyInterruptComponent {
    button0: Button0Pin,
    button1: Button1Pin,
    button2: Button2Pin,
    button3: Button3Pin,
    alarm: Alarm0,
    button0_has_edge_high: bool,
    button1_has_edge_high: bool,
    button2_has_edge_high: bool,
    button3_has_edge_high: bool,
}

/// This how we transfer our Led and Button pins into the Interrupt Handler.
/// We'll have the option hold both using the LedAndButton type.
/// This will make it a bit easier to unpack them later.
static GLOBAL_KEY_INTERRUPT_COMPONENT: Mutex<RefCell<Option<KeyInterruptComponent>>> =
    Mutex::new(RefCell::new(None));

enum Buttons {
    Button0,
    Button1,
    Button2,
    Button3,
}

static GLOBAL_BUTTON_PRESSED_QUEUE: Mutex<RefCell<TODO>> = Mutex::new(RefCell::new(false));

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

    // Set up the GPIO pin that will be our input
    let button0 = pins.gpio19.reconfigure();
    let button1 = pins.gpio18.reconfigure();
    let button2 = pins.gpio17.reconfigure();
    let button3 = pins.gpio16.reconfigure();
    button0.set_interrupt_enabled(EdgeHigh, true);
    button1.set_interrupt_enabled(EdgeHigh, true);
    button2.set_interrupt_enabled(EdgeHigh, true);
    button3.set_interrupt_enabled(EdgeHigh, true);

    // Give away our pins by moving them into the `GLOBAL_PINS` variable.
    // We won't need to access them in the main thread again
    critical_section::with(|cs| {
        GLOBAL_KEY_INTERRUPT_COMPONENT
            .borrow(cs)
            .replace(Some(KeyInterruptComponent {
                button0,
                button1,
                button2,
                button3,
                alarm,
                button0_has_edge_high: false,
                button1_has_edge_high: false,
                button2_has_edge_high: false,
                button3_has_edge_high: false,
            }))
    });

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

    // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
    // will jump to the interrupt function when the interrupt occurs.
    // We do this last so that the interrupt can't go off while
    // it is in the middle of being configured
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

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
        critical_section::with(|cs| {
            if GLOBAL_BUTTON_PRESSED.borrow(cs).replace(false) {
                button_pressed = true;
            }
        });
        if button_pressed {
            counter += 1;
            writeln!(console, "c:{}", counter).unwrap();
        }

        timer.delay_ms(10);
        // interrupts handle everything else in this example.
        // cortex_m::asm::wfi();
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    debug!("IO_IRQ_BANK0");

    critical_section::with(|cs| {
        let mut binding = GLOBAL_KEY_INTERRUPT_COMPONENT.borrow_ref_mut(cs);
        let component = binding.as_mut().unwrap();
        // Check if the interrupt source is from the push button going from high-to-low.
        // Note: this will always be true in this example, as that is the only enabled GPIO interrupt source
        if component.button0.interrupt_status(EdgeHigh) {
            debug!("button0 EdgeHigh");
            component.button0.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button0_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }
        if component.button1.interrupt_status(EdgeHigh) {
            debug!("button1 EdgeHigh");
            component.button1.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button1_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }
    })
}

#[interrupt]
fn TIMER_IRQ_0() {
    debug!("TIMER_IRQ_0");

    critical_section::with(|cs| {
        let mut binding = GLOBAL_KEY_INTERRUPT_COMPONENT.borrow_ref_mut(cs);
        let button_pressed = GLOBAL_BUTTON_PRESSED.borrow(cs);
        let component = binding.as_mut().unwrap();

        if component.button0_has_edge_high {
            if component.button0.is_high().unwrap() {
                info!("BUTTON0 PRESSED!");
                button_pressed.replace(true);
            }
            component.button0_has_edge_high = false;
        }
        if component.button1_has_edge_high {
            if component.button1.is_high().unwrap() {
                info!("BUTTON1 PRESSED!");
                button_pressed.replace(true);
            }
            component.button1_has_edge_high = false;
        }

        component.alarm.clear_interrupt()
    })
}

// End of file
