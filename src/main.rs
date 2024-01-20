//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use rp_pico as bsp;

use embedded_hal::digital::v2::{InputPin, ToggleableOutputPin};

use bsp::hal::{
    clocks::init_clocks_and_plls, gpio, gpio::Interrupt::EdgeHigh, pac, sio::Sio,
    watchdog::Watchdog, Timer,
};

use bsp::hal::pac::interrupt;

use fugit::ExtU32;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;

use alloc_cortex_m::CortexMHeap;
use core::alloc::Layout;
use rp_pico::hal::timer::{Alarm, Alarm0};

extern crate alloc;

// Pin types quickly become very long!
// We'll create some type aliases using `type` to help with that

/// This pin will be our output - it will drive an LED if you run this on a Pico
type LedPin = gpio::Pin<gpio::bank0::Gpio25, gpio::FunctionSioOutput, gpio::PullNone>;

/// This pin will be our interrupt source.
/// ~~It will trigger an interrupt if pulled to ground (via a switch or jumper wire)~~
/// 電流が流れている時にONとしたいので、PullUp から PullDown に変更した
type ButtonPin = gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSioInput, gpio::PullDown>;
// type Button1Pin = gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSioInput, gpio::PullDown>;
// type Button2Pin = gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSioInput, gpio::PullDown>;
// type Button3Pin = gpio::Pin<gpio::bank0::Gpio16, gpio::FunctionSioInput, gpio::PullDown>;

/// Since we're always accessing these pins together we'll store them in a tuple.
/// Giving this tuple a type alias means we won't need to use () when putting them
/// inside an Option. That will be easier to read.
type LedAndButtonAndAlarm = (LedPin, ButtonPin, Alarm0);

/// This how we transfer our Led and Button pins into the Interrupt Handler.
/// We'll have the option hold both using the LedAndButton type.
/// This will make it a bit easier to unpack them later.
static GLOBAL_PINS: Mutex<RefCell<Option<LedAndButtonAndAlarm>>> = Mutex::new(RefCell::new(None));

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

    let led = pins.led.reconfigure(); // into_push_pull_output

    // Set up the GPIO pin that will be our input
    let in_pin = pins.gpio19.reconfigure();

    // Trigger on the 'falling edge' of the input pin.
    // This will happen as the button is being pressed
    // EdgeLow から EdgeHigh に変更した
    in_pin.set_interrupt_enabled(EdgeHigh, true);

    // Give away our pins by moving them into the `GLOBAL_PINS` variable.
    // We won't need to access them in the main thread again
    // critical_section::with(|cs| {
    //     GLOBAL_PINS.borrow(cs).replace(Some((led, in_pin)));
    // });
    cortex_m::interrupt::free(|cs| {
        GLOBAL_PINS.borrow(cs).replace(Some((led, in_pin, alarm)));
    });

    // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
    // will jump to the interrupt function when the interrupt occurs.
    // We do this last so that the interrupt can't go off while
    // it is in the middle of being configured
    unsafe {
        pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
    }

    loop {
        // interrupts handle everything else in this example.
        cortex_m::asm::wfi();
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    // The `#[interrupt]` attribute covertly converts this to `&'static mut Option<LedAndButton>`
    static mut LED_AND_BUTTON_AND_ALARM: Option<LedAndButtonAndAlarm> = None;

    // This is one-time lazy initialisation. We steal the variables given to us
    // via `GLOBAL_PINS`.
    if LED_AND_BUTTON_AND_ALARM.is_none() {
        cortex_m::interrupt::free(|cs| {
            *LED_AND_BUTTON_AND_ALARM = GLOBAL_PINS.borrow(cs).take();
        });
    }

    // Need to check if our Option<LedAndButtonPins> contains our pins
    if let Some(gpios) = LED_AND_BUTTON_AND_ALARM {
        // borrow led and button by *destructuring* the tuple
        // these will be of type `&mut LedPin` and `&mut ButtonPin`, so we don't have
        // to move them back into the static after we use them
        let (_, button, alarm) = gpios;
        // Check if the interrupt source is from the push button going from high-to-low.
        // Note: this will always be true in this example, as that is the only enabled GPIO interrupt source
        if button.interrupt_status(EdgeHigh) {
            info!("EdgeHigh");
            // toggle can't fail, but the embedded-hal traits always allow for it
            // we can discard the return value by assigning it to an unnamed variable
            // let _ = led.toggle();

            // Our interrupt doesn't clear itself.
            // Do that now so we don't immediately jump back to this interrupt handler.
            button.clear_interrupt(EdgeHigh);

            alarm.schedule(10.millis()).unwrap();
        }
    }
}

#[interrupt]
fn TIMER_IRQ_0() {
    static mut LED_AND_BUTTON_AND_ALARM: Option<LedAndButtonAndAlarm> = None;
    info!("TIMER_IRQ_0");

    if LED_AND_BUTTON_AND_ALARM.is_none() {
        info!("is_none");

        cortex_m::interrupt::free(|cs| {
            *LED_AND_BUTTON_AND_ALARM = GLOBAL_PINS.borrow(cs).take();
        });
    }

    if let Some(gpios) = LED_AND_BUTTON_AND_ALARM {
        let (led, button, alarm) = gpios;
        if button.is_high().unwrap() {
            led.toggle().unwrap();
        }
        alarm.clear_interrupt()
    } else {
        info!("what happen?");
    }
}

// End of file
