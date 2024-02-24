use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use defmt::info;
use defmt::{Debug2Format, Format};

use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;

use core::marker::Copy;

type Led0Pin = gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSioOutput, gpio::PullDown>;
type Led1Pin = gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSioOutput, gpio::PullDown>;
type Led2Pin = gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSioOutput, gpio::PullDown>;
type Led3Pin = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;

type LedPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;

use led_pins::{Duration, Instant, Led, LedMode, LedPins, LedStatus, Scheduler};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct PicoInstant(rp_pico::hal::timer::Instant);
impl Instant for PicoInstant {
    fn add_millis(&self, millis: u32) -> Self {
        PicoInstant(self.0 + millis.millis())
    }
}

#[derive(Clone, Copy, Debug)]
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

impl core::fmt::Debug for PicoLed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PicoLed(...)")
    }
}

use rp_pico::hal::timer::Alarm as _;
struct PicoScheduler {
    timer: rp_pico::hal::Timer,
    alarm: rp_pico::hal::timer::Alarm1,
}
impl Scheduler<PicoInstant, PicoDuration> for PicoScheduler {
    fn get_counter(&self) -> PicoInstant {
        PicoInstant(self.timer.get_counter())
    }

    fn finished(&self) -> bool {
        self.alarm.finished()
    }
    fn schedule(&mut self, countdown: PicoDuration) {
        self.alarm.schedule(countdown.0).unwrap();
    }
    fn schedule_at(&mut self, at: PicoInstant) {
        self.alarm.schedule_at(at.0).unwrap();
    }
    fn clear_interrupt(&mut self) {
        self.alarm.clear_interrupt()
    }
}

impl core::fmt::Debug for PicoScheduler {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PicoScheduler(...)")
    }
}

type GlobalLedPins = LedPins<PicoInstant, PicoDuration, PicoLed, PicoScheduler>;

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
                PicoScheduler { timer, alarm },
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

#[derive(Format)]
struct DebugLedPins<'a>(#[defmt(Debug2Format)] &'a GlobalLedPins);

pub fn debug_print() {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        info!("{}", DebugLedPins(component));
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
