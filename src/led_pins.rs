use defmt::{error, write};

use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm1, Instant};
use rp_pico::hal::Timer;

use core::marker::Copy;
use fixed_size_priority_queue::FixedSizePriorityQueue;

type Led0Pin = gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSioOutput, gpio::PullDown>;
type Led1Pin = gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSioOutput, gpio::PullDown>;
type Led2Pin = gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSioOutput, gpio::PullDown>;
type Led3Pin = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;

type LedPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;

#[derive(Clone, Copy, PartialEq, Eq, Ord)]
struct ScheduledPinsCommand {
    schedule: Instant,
    led_num: usize,
    command: Command,
}

impl PartialOrd for ScheduledPinsCommand {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.schedule.partial_cmp(&other.schedule)
    }
}

impl defmt::Format for ScheduledPinsCommand {
    fn format(&self, fmt: defmt::Formatter) {
        write!(
            fmt,
            "[{}, {}, {}]",
            self.schedule.ticks(),
            self.led_num,
            self.command
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
enum Command {
    ChangeMode(Mode),
    ChangePin(Pin), // BLINKモードの時のみピンの変更がタイマーでくる
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
pub enum Mode {
    HIGH,
    LOW,
    BLINK,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
enum Pin {
    HIGH,
    LOW,
}

pub struct LedPinsComponent {
    leds: [LedPin; 4],
    led_modes: [Mode; 4],
    timer: Timer,
    alarm: Alarm1,
    queue: FixedSizePriorityQueue<ScheduledPinsCommand, 20>,
}

static GLOBAL_LED_PINS_COMPONENT: Mutex<RefCell<Option<LedPinsComponent>>> =
    Mutex::new(RefCell::new(None));

impl LedPinsComponent {
    pub fn init(
        led0: Led0Pin,
        led1: Led1Pin,
        led2: Led2Pin,
        led3: Led3Pin,
        timer: Timer,
        mut alarm: Alarm1,
    ) {
        alarm.enable_interrupt();

        critical_section::with(|cs| {
            GLOBAL_LED_PINS_COMPONENT
                .borrow(cs)
                .replace(Some(LedPinsComponent {
                    leds: [
                        led0.into_dyn_pin(),
                        led1.into_dyn_pin(),
                        led2.into_dyn_pin(),
                        led3.into_dyn_pin(),
                    ],
                    led_modes: [Mode::LOW; 4],
                    timer,
                    alarm,
                    queue: FixedSizePriorityQueue::new(),
                }))
        });

        unsafe {
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
        }
    }

    pub fn set_mode(led_num: usize, mode: Mode) {
        critical_section::with(|cs| {
            if led_num > 4 {
                error!("invalid led_num: {}", led_num);
                return;
            }

            let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
            let component = binding.as_mut().unwrap();

            if let Some(next) = component._change_mode(led_num, mode) {
                component.queue.push(next);
            }
        })
    }

    pub fn set_mode_later(led_num: usize, mode: Mode, countdown: MicrosDurationU32) {
        critical_section::with(|cs| {
            if led_num > 4 {
                error!("invalid led_num: {}", led_num);
                return;
            }

            let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
            let component = binding.as_mut().unwrap();

            component.queue.push(ScheduledPinsCommand {
                schedule: component.timer.get_counter() + countdown,
                led_num,
                command: Command::ChangeMode(mode),
            });
            if component.alarm.finished() {
                component.alarm.schedule(countdown).unwrap();
            }
        })
    }

    // モード切り替え HIGHとLOWは即座にピンの状態を変えるが、BLINKの場合次に動かすコマンドを返す
    fn _change_mode(&mut self, led_num: usize, mode: Mode) -> Option<ScheduledPinsCommand> {
        self.led_modes[led_num] = mode;
        match mode {
            Mode::HIGH => {
                self.leds[led_num].set_high().unwrap();
                None
            }
            Mode::LOW => {
                self.leds[led_num].set_low().unwrap();
                None
            }
            Mode::BLINK => Some(ScheduledPinsCommand {
                schedule: self.timer.get_counter() + 250.millis(),
                led_num,
                command: Command::ChangePin(Pin::HIGH),
            }),
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
            (Command::ChangeMode(mode), _) => {
                self._change_mode(led_num, mode);
                None
            }
            // ピン切り替えはBLINKの時だけ扱う
            (Command::ChangePin(pin), Mode::BLINK) => {
                if pin == Pin::HIGH {
                    self.leds[led_num].set_high().unwrap();
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter() + 250.millis(),
                        led_num,
                        command: Command::ChangePin(Pin::LOW),
                    })
                } else {
                    self.leds[led_num].set_low().unwrap();
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter() + 250.millis(),
                        led_num,
                        command: Command::ChangePin(Pin::HIGH),
                    })
                }
            }
            (Command::ChangePin(_), _) => None,
        }
    }
}

#[interrupt]
fn TIMER_IRQ_1() {
    critical_section::with(|cs| {
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();

        let now = component.timer.get_counter();

        // キューに溜まったもののうち現在より前のものは全て実行
        while let Some(&next) = component.queue.peek() {
            if next.schedule <= now {
                let _ = component.queue.pop();
                if let Some(next) = component._handle_command(next.led_num, next.command) {
                    component.queue.push(next);
                }
            } else {
                break;
            }
        }

        // キューに残りがあれば、タイマーセット
        if let Some(next) = component.queue.peek() {
            component.alarm.schedule_at(next.schedule).unwrap();
        }
        component.alarm.clear_interrupt();
    })
}
