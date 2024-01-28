use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;

use core::marker::Copy;

type Led0Pin = gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSioOutput, gpio::PullDown>;
type Led1Pin = gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSioOutput, gpio::PullDown>;
type Led2Pin = gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSioOutput, gpio::PullDown>;
type Led3Pin = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;

type LedPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;

use led_pins::{Alarm, Duration, Instant, Led, LedMode, LedPins, LedStatus, Timer};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
struct PicoInstant(rp_pico::hal::timer::Instant);
impl Instant for PicoInstant {
    fn add_millis(&self, millis: u32) -> Self {
        PicoInstant(self.0 + millis.millis())
    }
}

#[derive(Clone, Copy)]
struct PicoDuration(fugit::MicrosDurationU32);
impl Duration for PicoDuration {
    fn from_millis(millis: u32) -> Self {
        PicoDuration(millis.millis())
    }
    fn to_millis(&self) -> u32 {
        self.0.to_millis()
    }
}

struct PicoLed(LedPin);
impl Led for PicoLed {
    #[cfg(test)]
    fn get_status(&self) -> LedStatus {
        todo!()
    }
    fn set_status(&mut self, led_status: LedStatus) {
        match led_status {
            LedStatus::HIGH => self.0.set_high().unwrap(),
            LedStatus::LOW => self.0.set_low().unwrap(),
        }
    }
}

struct PicoTimer(rp_pico::hal::Timer);
impl Timer<PicoInstant> for PicoTimer {
    fn get_counter(&self) -> PicoInstant {
        PicoInstant(self.0.get_counter())
    }
}

use rp_pico::hal::timer::Alarm as _;
struct PicoAlarm(rp_pico::hal::timer::Alarm1);
impl Alarm<PicoInstant, PicoDuration> for PicoAlarm {
    fn finished(&self) -> bool {
        self.0.finished()
    }
    fn schedule(&mut self, countdown: PicoDuration) {
        self.0.schedule(countdown.0).unwrap();
    }
    fn schedule_at(&mut self, at: PicoInstant) {
        self.0.schedule_at(at.0).unwrap();
    }
    fn clear_interrupt(&mut self) {
        self.0.clear_interrupt()
    }
}

type GlobalLedPins = LedPins<PicoInstant, PicoDuration, PicoLed, PicoTimer, PicoAlarm>;

static GLOBAL_LED_PINS_COMPONENT: Mutex<RefCell<Option<GlobalLedPins>>> =
    Mutex::new(RefCell::new(None));

pub fn init(
    led0: Led0Pin,
    led1: Led1Pin,
    led2: Led2Pin,
    led3: Led3Pin,
    timer: rp_pico::hal::Timer,
    mut alarm: rp_pico::hal::timer::Alarm1,
) {
    use rp_pico::hal::timer::Alarm as _A;
    alarm.enable_interrupt();

    critical_section::with(|cs| {
        GLOBAL_LED_PINS_COMPONENT
            .borrow(cs)
            .replace(Some(LedPins::init(
                PicoLed(led0.into_dyn_pin()),
                PicoLed(led1.into_dyn_pin()),
                PicoLed(led2.into_dyn_pin()),
                PicoLed(led3.into_dyn_pin()),
                PicoTimer(timer),
                PicoAlarm(alarm),
            )))
    });

    unsafe {
        pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
    }
}

pub fn set_led_mode(led_num: usize, led_mode: LedMode) {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        component.set_led_mode(led_num, led_mode)
    })
}

pub fn set_mode_later(led_num: usize, led_mode: LedMode, countdown: MicrosDurationU32) {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        component.set_mode_later(led_num, led_mode, PicoDuration(countdown))
    })
}

#[interrupt]
fn TIMER_IRQ_1() {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        component.handle_schedule();
    })
}
