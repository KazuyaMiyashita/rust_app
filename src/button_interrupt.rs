use defmt::{debug, error, info};

use bsp::hal::fugit::{ExtU32, RateExtU32};
use bsp::hal::{gpio, gpio::Interrupt::EdgeHigh, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm0};

/// This how we transfer our Led and Button pins into the Interrupt Handler.
/// We'll have the option hold both using the LedAndButton type.
/// This will make it a bit easier to unpack them later.
static GLOBAL_KEY_INTERRUPT_COMPONENT: Mutex<RefCell<Option<KeyInterruptComponent>>> =
    Mutex::new(RefCell::new(None));

static GLOBAL_BUTTON_PRESSED_QUEUE: Mutex<RefCell<TODO>> = Mutex::new(RefCell::new(false));

enum Buttons {
    Button0,
    Button1,
    Button2,
    Button3,
}

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

pub struct ButtonInterrupt {}
impl ButtonInterrupt {
    pub fn new(
        button0: Button0Pin,
        button1: Button1Pin,
        button2: Button2Pin,
        button3: Button3Pin,
    ) -> Self {
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

        // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
        // will jump to the interrupt function when the interrupt occurs.
        // We do this last so that the interrupt can't go off while
        // it is in the middle of being configured
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        }

        ButtonInterrupt {}
    }

    pub fn pop(&self) -> TODO {
        critical_section::with(|cs| {
            if GLOBAL_BUTTON_PRESSED.borrow(cs).replace(false) {
                button_pressed = true;
            }
        });
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
