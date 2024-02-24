use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;

use bsp::hal::timer::Instant;
use core::marker::Copy;
use core::ops::Add;
use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32 as Duration;
use rp_pico as bsp;

type Led0Pin = gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSioOutput, gpio::PullDown>;
type Led1Pin = gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSioOutput, gpio::PullDown>;
type Led2Pin = gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSioOutput, gpio::PullDown>;
type Led3Pin = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;

type LedPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;

use rp_pico::hal::timer::Alarm as _;

static GLOBAL_LED_PINS_COMPONENT: Mutex<RefCell<Option<LedPins>>> = Mutex::new(RefCell::new(None));

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
                led0.into_dyn_pin(),
                led1.into_dyn_pin(),
                led2.into_dyn_pin(),
                led3.into_dyn_pin(),
                timer,
                alarm,
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

pub fn set_mode_later(led_num: usize, led_mode: LedMode, countdown: Duration) {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        component.set_mode_later(led_num, led_mode, countdown)
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

//

use fixed_size_priority_queue::FixedSizePriorityQueue;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
struct ScheduledPinsCommand {
    schedule: Instant,
    led_num: usize,
    command: Command,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Command {
    ChangeLedMode(LedMode),
    ChangeLedStatus(LedStatus), // BLINKモードの時のみピンの変更がタイマーでくる
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LedMode {
    HIGH,
    LOW,
    BLINK,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LedStatus {
    HIGH,
    LOW,
}

pub struct LedPins {
    leds: [LedPin; 4],
    led_modes: [LedMode; 4],
    queue: FixedSizePriorityQueue<ScheduledPinsCommand, 20>,
    timer: rp_pico::hal::Timer,
    alarm: rp_pico::hal::timer::Alarm1,
}

impl LedPins {
    fn blink_millis() -> Duration {
        100.millis()
    }

    pub fn init(
        led0: LedPin,
        led1: LedPin,
        led2: LedPin,
        led3: LedPin,
        timer: rp_pico::hal::Timer,
        alarm: rp_pico::hal::timer::Alarm1,
    ) -> Self {
        LedPins {
            leds: [led0, led1, led2, led3],
            led_modes: [LedMode::LOW; 4],
            queue: FixedSizePriorityQueue::new(),
            timer,
            alarm,
        }
    }

    pub fn set_led_mode(&mut self, led_num: usize, led_mode: LedMode) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }
        if let Some(next) = self._change_mode(led_num, led_mode) {
            self.queue.push(next);
            if self.alarm.finished() {
                self.alarm.schedule_at(next.schedule).unwrap();
            }
        }
    }

    pub fn set_mode_later(&mut self, led_num: usize, led_mode: LedMode, countdown: Duration) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }

        self.queue.push(ScheduledPinsCommand {
            schedule: self.timer.get_counter().add(countdown),
            led_num,
            command: Command::ChangeLedMode(led_mode),
        });
        if self.alarm.finished() {
            self.alarm.schedule(countdown).unwrap();
        }
    }

    // モード切り替え HIGHとLOWは即座にピンの状態を変えるが、BLINKの場合次に動かすコマンドを返す
    fn _change_mode(&mut self, led_num: usize, led_mode: LedMode) -> Option<ScheduledPinsCommand> {
        self.led_modes[led_num] = led_mode;
        match led_mode {
            LedMode::HIGH => {
                self.leds[led_num].set_high().unwrap();
                None
            }
            LedMode::LOW => {
                self.leds[led_num].set_low().unwrap();
                None
            }
            LedMode::BLINK => {
                self.leds[led_num].set_high().unwrap();
                Some(ScheduledPinsCommand {
                    schedule: self.timer.get_counter().add(Self::blink_millis()),
                    led_num,
                    command: Command::ChangeLedStatus(LedStatus::LOW),
                })
            }
        }
    }

    // ピン切り替えかモード切り替えがやってくる。次にスケジュールするものがあればそれを返す
    fn _handle_command(
        &mut self,
        led_num: usize,
        command: Command,
    ) -> Option<ScheduledPinsCommand> {
        let current_mode = self.led_modes[led_num];
        match (command, current_mode) {
            (Command::ChangeLedMode(mode), _) => {
                self._change_mode(led_num, mode);
                None
            }
            // ピン切り替えはBLINKの時だけ扱う
            (Command::ChangeLedStatus(pin), LedMode::BLINK) => {
                if pin == LedStatus::HIGH {
                    self.leds[led_num].set_high().unwrap();
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter().add(Self::blink_millis()),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::LOW),
                    })
                } else {
                    self.leds[led_num].set_low().unwrap();
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter().add(Self::blink_millis()),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::HIGH),
                    })
                }
            }
            (Command::ChangeLedStatus(_), _) => None,
        }
    }

    pub fn handle_schedule(&mut self) {
        self.alarm.clear_interrupt();
        let now = self.timer.get_counter();

        // キューに溜まったもののうち現在より前のものは全て実行
        while let Some(&next) = self.queue.peek() {
            if next.schedule <= now {
                let _ = self.queue.pop();
                if let Some(next) = self._handle_command(next.led_num, next.command) {
                    self.queue.push(next);
                }
            } else {
                break;
            }
        }

        // タイマーが完了していて、キューに残りがあれば、タイマーセット
        if self.alarm.finished() {
            if let Some(next) = self.queue.peek() {
                self.alarm.schedule_at(next.schedule).unwrap();
            }
        }
    }
}
