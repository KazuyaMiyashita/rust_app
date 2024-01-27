use defmt::{debug, error, info, write};

use bsp::hal::{gpio, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::OutputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm1, Instant};
use rp_pico::hal::Timer;

use core::marker::Copy;

type Led0Pin = gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSioOutput, gpio::PullDown>;
type Led1Pin = gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSioOutput, gpio::PullDown>;
type Led2Pin = gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSioOutput, gpio::PullDown>;
type Led3Pin = gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSioOutput, gpio::PullDown>;

type LedPin = gpio::Pin<gpio::DynPinId, gpio::FunctionSioOutput, gpio::PullDown>;

struct FixedSizeQueue<T, const N: usize> {
    heap: [Option<T>; N],
    size: usize,
}

impl<T, const N: usize> FixedSizeQueue<T, N>
where
    T: Copy + Ord + Sized + defmt::Format,
{
    pub fn new() -> Self {
        FixedSizeQueue {
            heap: [None; N],
            size: 0,
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.heap.get(0).and_then(|opt| opt.as_ref())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size > 0 {
            let root = self.heap[0].take();
            self.size -= 1;
            self.heapify_down(0);
            root
        } else {
            None
        }
    }

    pub fn push(&mut self, item: T) {
        debug!(
            "FixedSizeQueue::push. item: {}, current heap: {}",
            item, self.heap
        );
        if self.size < N {
            self.heap[self.size] = Some(item);
            self.size += 1;
            self.heapify_up(self.size - 1);
        } else if let Some(root) = self.heap.get_mut(0) {
            if let Some(top) = root.as_mut() {
                if *top < item {
                    *top = item;
                    self.heapify_down(0);
                }
            }
        }
    }

    fn heapify_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.heap[index] > self.heap[parent] {
                self.heap.swap(index, parent);
                index = parent;
            } else {
                break;
            }
        }
    }

    fn heapify_down(&mut self, mut index: usize) {
        while 2 * index + 1 < self.size {
            let left_child = 2 * index + 1;
            let right_child = 2 * index + 2;
            let mut largest_child = left_child;

            if right_child < self.size && self.heap[right_child] > self.heap[left_child] {
                largest_child = right_child;
            }

            if self.heap[index] < self.heap[largest_child] {
                self.heap.swap(index, largest_child);
                index = largest_child;
            } else {
                break;
            }
        }
    }
}

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
