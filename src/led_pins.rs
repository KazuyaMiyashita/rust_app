use defmt::{error, info, write};

use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm1, Instant};
use rp_pico::hal::Timer;

use core::marker::Copy;
use fixed_size_queue::FixedSizeQueue;

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
        other.schedule.partial_cmp(&self.schedule)
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
}

pub struct LedPinsComponent {
    leds: [LedPin; 4],
    timer: Timer,
    alarm: Alarm1,
    queue: FixedSizeQueue<ScheduledPinsCommand, 5>,
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
                    timer,
                    alarm,
                    queue: FixedSizeQueue::new(),
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
                info!("led alarm is finished. new alarm set.");
                component.alarm.schedule(countdown).unwrap();
            }
        })
    }

    fn do_pins_command(&mut self, pins_command: PinsCommand) {
        match pins_command.command {
            Command::HIGH => self.leds[pins_command.led_num].set_high().unwrap(),
            Command::LOW => self.leds[pins_command.led_num].set_low().unwrap(),
        }
    }
}

#[interrupt]
fn TIMER_IRQ_1() {
    critical_section::with(|cs| {
        info!("TIMER_IRQ_1");
        let mut binding = GLOBAL_LED_PINS_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();

        if let Some(scheduled_pins_command) = component.queue.pop() {
            info!("TIMER_IRQ_1 poped command: {}", scheduled_pins_command);
            component.do_pins_command(scheduled_pins_command.pins_command);
        } else {
            info!("TIMER_IRQ_1 no command found. why?");
        }

        if let Some(next) = component.queue.peek() {
            info!("next queue is found. {}", next);
            component.alarm.schedule_at(next.schedule).unwrap();
        } else {
            info!("next queue is none. clear interrupt");
            component.alarm.clear_interrupt();
        }
    })
}
