#![cfg_attr(not(test), no_std)]

use core::marker::{Copy, PhantomData};
use fixed_size_priority_queue::FixedSizePriorityQueue;

pub trait Instant {
    fn add_millis(&self, millis: u32) -> Self;
}
pub trait Duration: Copy {
    fn from_millis(millis: u32) -> Self;
    fn to_millis(&self) -> u32;
}

pub trait Led {
    #[cfg(test)]
    fn get_status(&self) -> LedStatus;

    fn set_status(&mut self, led_status: LedStatus);
}

pub trait Timer<I: Instant> {
    fn get_counter(&self) -> I;
}
pub trait Alarm<I: Instant, D: Duration> {
    fn finished(&self) -> bool;
    fn schedule(&mut self, countdown: D);
    fn schedule_at(&mut self, at: I);
    fn clear_interrupt(&mut self);
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(test, derive(Debug))]
struct ScheduledPinsCommand<I: Instant + Ord + Copy> {
    schedule: I,
    led_num: usize,
    command: Command,
}

impl<I: Instant + Ord + Copy> Ord for ScheduledPinsCommand<I> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.schedule.cmp(&other.schedule)
    }
}

impl<I: Instant + Ord + Copy> PartialOrd for ScheduledPinsCommand<I> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.schedule.partial_cmp(&other.schedule)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]

enum Command {
    ChangeLedMode(LedMode),
    ChangeLedStatus(LedStatus), // BLINKモードの時のみピンの変更がタイマーでくる
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]

pub enum LedMode {
    HIGH,
    LOW,
    BLINK,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
pub enum LedStatus {
    HIGH,
    LOW,
}

pub struct LedPins<I: Instant + Ord + Copy, D: Duration, L: Led, T: Timer<I>, A: Alarm<I, D>> {
    leds: [L; 4],
    led_modes: [LedMode; 4],
    queue: FixedSizePriorityQueue<ScheduledPinsCommand<I>, 20>,
    timer: T,
    alarm: A,
    _phantom: PhantomData<D>,
}

impl<I, D, L, T, A> LedPins<I, D, L, T, A>
where
    I: Instant + Ord + Copy,
    D: Duration,
    L: Led,
    T: Timer<I>,
    A: Alarm<I, D>,
{
    pub fn init(led0: L, led1: L, led2: L, led3: L, timer: T, alarm: A) -> Self {
        LedPins {
            leds: [led0, led1, led2, led3],
            led_modes: [LedMode::LOW; 4],
            queue: FixedSizePriorityQueue::new(),
            timer,
            alarm,
            _phantom: PhantomData {},
        }
    }

    pub fn set_led_mode(&mut self, led_num: usize, led_mode: LedMode) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }
        if let Some(next) = self._change_mode(led_num, led_mode) {
            self.queue.push(next);
            self.alarm.schedule_at(next.schedule)
        }
    }

    pub fn set_mode_later(&mut self, led_num: usize, led_mode: LedMode, countdown: D) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }

        self.queue.push(ScheduledPinsCommand {
            schedule: self.timer.get_counter().add_millis(countdown.to_millis()),
            led_num,
            command: Command::ChangeLedMode(led_mode),
        });
        if self.alarm.finished() {
            self.alarm.schedule(countdown);
        }
    }

    // モード切り替え HIGHとLOWは即座にピンの状態を変えるが、BLINKの場合次に動かすコマンドを返す
    fn _change_mode(
        &mut self,
        led_num: usize,
        led_mode: LedMode,
    ) -> Option<ScheduledPinsCommand<I>> {
        self.led_modes[led_num] = led_mode;
        match led_mode {
            LedMode::HIGH => {
                self.leds[led_num].set_status(LedStatus::HIGH);
                None
            }
            LedMode::LOW => {
                self.leds[led_num].set_status(LedStatus::LOW);
                None
            }
            LedMode::BLINK => Some(ScheduledPinsCommand {
                schedule: self.timer.get_counter().add_millis(250),
                led_num,
                command: Command::ChangeLedStatus(LedStatus::HIGH),
            }),
        }
    }

    // ピン切り替えかモード切り替えがやってくる。次にスケジュールするものがあればそれを返す
    fn _handle_command(
        &mut self,
        led_num: usize,
        command: Command,
    ) -> Option<ScheduledPinsCommand<I>> {
        let current_mode = self.led_modes[led_num];
        match (command, current_mode) {
            (Command::ChangeLedMode(mode), _) => {
                self._change_mode(led_num, mode);
                None
            }
            // ピン切り替えはBLINKの時だけ扱う
            (Command::ChangeLedStatus(pin), LedMode::BLINK) => {
                if pin == LedStatus::HIGH {
                    self.leds[led_num].set_status(LedStatus::HIGH);
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter().add_millis(250),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::LOW),
                    })
                } else {
                    self.leds[led_num].set_status(LedStatus::LOW);
                    Some(ScheduledPinsCommand {
                        schedule: self.timer.get_counter().add_millis(250),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::HIGH),
                    })
                }
            }
            (Command::ChangeLedStatus(_), _) => None,
        }
    }

    pub fn handle_schedule(&mut self) {
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

        // キューに残りがあれば、タイマーセット
        if let Some(next) = self.queue.peek() {
            self.alarm.schedule_at(next.schedule);
        }
        self.alarm.clear_interrupt();
    }

    #[cfg(test)]
    fn led0(&mut self) -> &mut L {
        &mut self.leds[0]
    }

    #[cfg(test)]
    fn timer(&mut self) -> &mut T {
        &mut self.timer
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    impl Instant for u32 {
        fn add_millis(&self, millis: u32) -> Self {
            self + millis
        }
    }
    impl Duration for u32 {
        fn from_millis(millis: u32) -> Self {
            millis
        }
        fn to_millis(&self) -> u32 {
            *self
        }
    }

    struct MockLed {
        led_status: LedStatus,
    }
    impl MockLed {
        fn new() -> Self {
            MockLed {
                led_status: LedStatus::LOW,
            }
        }
    }
    impl Led for MockLed {
        fn get_status(&self) -> LedStatus {
            self.led_status
        }
        fn set_status(&mut self, led_status: LedStatus) {
            self.led_status = led_status;
        }
    }
    struct MockTimer {
        tick: u32,
    }
    impl MockTimer {
        fn new() -> Self {
            MockTimer { tick: 0 }
        }
    }
    impl Timer<u32> for MockTimer {
        fn get_counter(&self) -> u32 {
            self.tick
        }
    }

    struct MockAlarm {}
    impl MockAlarm {
        fn new() -> Self {
            MockAlarm {}
        }
    }
    impl Alarm<u32> for MockAlarm {
        fn finished(&self) -> bool {
            true
        }
        fn schedule<D: Duration>(&mut self, countdown: D) {}
        fn schedule_at(&mut self, at: u32) {}
        fn clear_interrupt(&mut self) {}
    }

    #[test]
    fn test1() {
        let mut led_pins: LedPins<u32, u32, MockLed, MockTimer, MockAlarm> = LedPins::init(
            MockLed::new(),
            MockLed::new(),
            MockLed::new(),
            MockLed::new(),
            MockTimer::new(),
            MockAlarm::new(),
        );

        led_pins.set_led_mode(0, LedMode::BLINK);
        assert_eq!(
            led_pins.queue.peek(),
            Some(ScheduledPinsCommand {
                schedule: 250,
                led_num: 0,
                command: Command::ChangeLedStatus(LedStatus::HIGH)
            })
            .as_ref()
        );

        led_pins.handle_schedule();
        assert_eq!(led_pins.led0().get_status(), LedStatus::LOW);

        led_pins.timer().tick = 250;
        led_pins.handle_schedule();
        assert_eq!(led_pins.led0().get_status(), LedStatus::HIGH);

        todo!("some test yet implemented")
    }
}
