use defmt::{error, info, write};

use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::{InputPin, OutputPin};
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
    pins_command: PinsCommand,
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
            self.pins_command.led_num,
            self.pins_command.command
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
struct PinsCommand {
    led_num: usize,
    command: Command,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, defmt::Format)]
pub enum Command {
    HIGH,
    LOW,
    BLINK,
}

pub struct LedPinsComponent {
    leds: [LedPin; 4],
    led_modes: [Command; 4],
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
                    led_modes: [Command::BLINK; 4],
                    timer,
                    alarm,
                    queue: FixedSizePriorityQueue::new(),
                }))
        });

        unsafe {
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
        }
    }

    pub fn set(led_num: usize, command: Command) {
        critical_section::with(|cs| {
            info!(
                "LedPinsComponent::set led_num: {}, command: {}",
                led_num, command
            );

            let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
            let component = binding.as_mut().unwrap();

            if led_num > 4 {
                error!("invalid led_num] {}", led_num);
                panic!("invalid led_num] {}", led_num);
            }

            component.do_pins_command(PinsCommand { led_num, command });
            // if let Some(next) = component.do_pins_command(PinsCommand { led_num, command }) {
            //     component.queue.push(next);
            // }
        })
    }

    pub fn set_later(led_num: usize, command: Command, countdown: MicrosDurationU32) {
        critical_section::with(|cs| {
            info!(
                "LedPinsComponent::set_later led_num: {}, command: {}",
                led_num, command
            );

            let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
            let component = binding.as_mut().unwrap();

            if led_num > 4 {
                error!("invalid led_num] {}", led_num);
                panic!("invalid led_num] {}", led_num);
            }

            component.queue.push(ScheduledPinsCommand {
                schedule: component.timer.get_counter() + countdown,
                pins_command: PinsCommand { led_num, command },
            });
            if component.alarm.finished() {
                component.alarm.schedule(countdown).unwrap();
            }
        })
    }

    // ピンの状態を変更し、次にスケジュールするものがあればそれを返す
    fn do_pins_command(&mut self, pins_command: PinsCommand) -> Option<ScheduledPinsCommand> {
        info!("Do {}", pins_command);

        let next = match pins_command.command {
            Command::HIGH => {
                self.leds[pins_command.led_num].set_high().unwrap();
                None
            }
            Command::LOW => {
                self.leds[pins_command.led_num].set_low().unwrap();
                None
            }
            Command::BLINK => {
                if self.leds[pins_command.led_num].is_high().unwrap() {
                    self.leds[pins_command.led_num].set_low().unwrap();
                } else {
                    self.leds[pins_command.led_num].set_high().unwrap();
                }

                // 現在のモードがBLINKの時に限り次のスケジュールを返す
                // (BLINKが送られてきても、現在の設定がLOWになっていることがある)
                if self.led_modes[pins_command.led_num] == Command::BLINK {
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter() + 250.millis(),
                        pins_command,
                    })
                } else {
                    None
                }
            }
        };
        self.led_modes[pins_command.led_num] = pins_command.command;
        next
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
                component.do_pins_command(next.pins_command);
                // if let Some(next) = component.do_pins_command(next.pins_command) {
                //     component.queue.push(next);
                // }
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
