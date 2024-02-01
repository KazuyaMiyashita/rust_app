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

pub trait Scheduler<I: Instant, D: Duration, F>
where
    F: Fn() -> (),
{
    // for Timer
    fn get_counter(&self) -> I;

    // for Alarm
    fn finished(&self) -> bool;
    fn schedule(&mut self, countdown: D);
    fn schedule_at(&mut self, at: I);
    fn clear_interrupt(&mut self);

    // other
    fn set_callback(&mut self, f: F);
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

pub struct LedPins<'a, I, D, L, F, S>
where
    I: Instant + Ord + Copy,
    D: Duration,
    L: Led,
    F: Fn() -> (),
    S: Scheduler<I, D, F>,
{
    leds: [L; 4],
    led_modes: [LedMode; 4],
    queue: FixedSizePriorityQueue<ScheduledPinsCommand<I>, 20>,
    scheduler: &'a mut S,
    _phantom_d: PhantomData<D>,
    _phantom_f: PhantomData<F>,
}

#[cfg(test)]
impl<'a, I, D, L, F, S> std::fmt::Debug for LedPins<'a, I, D, L, F, S>
where
    I: Instant + Ord + Copy + std::fmt::Debug,
    D: Duration + std::fmt::Debug,
    L: Led + std::fmt::Debug,
    F: Fn() -> (),
    S: Scheduler<I, D, F>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedPins")
            .field("leds", &self.leds)
            .field("led_modes", &self.led_modes)
            .field("queue", &self.queue)
            .field("scheduler", &format_args!("Fn() -> ()")) // クロージャ型のデバッグ表示
            .finish()
    }
}

impl<'a, I, D, L, F, S> LedPins<'a, I, D, L, F, S>
where
    I: Instant + Ord + Copy,
    D: Duration,
    L: Led,
    F: Fn() -> (),
    S: Scheduler<I, D, F>,
{
    pub fn init(led0: L, led1: L, led2: L, led3: L, scheduler: &'a mut S) -> Self {
        LedPins {
            leds: [led0, led1, led2, led3],
            led_modes: [LedMode::LOW; 4],
            queue: FixedSizePriorityQueue::new(),
            scheduler,
            _phantom_d: PhantomData {},
            _phantom_f: PhantomData {},
        }
    }

    pub fn set_led_mode(&mut self, led_num: usize, led_mode: LedMode) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }
        if let Some(next) = self._change_mode(led_num, led_mode) {
            self.queue.push(next);
            self.scheduler.schedule_at(next.schedule)
        }
    }

    pub fn set_mode_later(&mut self, led_num: usize, led_mode: LedMode, countdown: D) {
        if led_num > 4 {
            panic!("invalid led_num: {}", led_num);
        }

        self.queue.push(ScheduledPinsCommand {
            schedule: self
                .scheduler
                .get_counter()
                .add_millis(countdown.to_millis()),
            led_num,
            command: Command::ChangeLedMode(led_mode),
        });
        if self.scheduler.finished() {
            self.scheduler.schedule(countdown);
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
            LedMode::BLINK => {
                self.leds[led_num].set_status(LedStatus::HIGH);
                Some(ScheduledPinsCommand {
                    schedule: self.scheduler.get_counter().add_millis(250),
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
                        schedule: self.scheduler.get_counter().add_millis(250),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::LOW),
                    })
                } else {
                    self.leds[led_num].set_status(LedStatus::LOW);
                    Some(ScheduledPinsCommand {
                        schedule: self.scheduler.get_counter().add_millis(250),
                        led_num,
                        command: Command::ChangeLedStatus(LedStatus::HIGH),
                    })
                }
            }
            (Command::ChangeLedStatus(_), _) => None,
        }
    }

    pub fn handle_schedule(&mut self) {
        self.scheduler.clear_interrupt();
        let now = self.scheduler.get_counter();

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
            self.scheduler.schedule_at(next.schedule);
        }
    }

    #[cfg(test)]
    fn leds_status(&mut self) -> [LedStatus; 4] {
        [
            self.leds[0].get_status(),
            self.leds[1].get_status(),
            self.leds[2].get_status(),
            self.leds[3].get_status(),
        ]
    }

    #[cfg(test)]
    fn scheduler(&mut self) -> &mut S {
        &mut self.scheduler
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

    #[derive(Clone, Copy, Debug)]
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

    struct MockScheduler<F>
    where
        F: Fn() -> (),
    {
        counter: u32,
        maybe_next_schedule: Option<u32>,
        maybe_callback: Option<Box<F>>,
    }
    impl<F> MockScheduler<F>
    where
        F: Fn() -> (),
    {
        fn new() -> Self {
            MockScheduler {
                counter: 0,
                maybe_next_schedule: None,
                maybe_callback: None,
            }
        }
        fn next(&mut self) {
            match self.maybe_next_schedule {
                Some(schedule) if schedule == self.counter => {
                    if let Some(callback) = &self.maybe_callback {
                        callback()
                    }
                    self.maybe_next_schedule = None;
                }
                _ => (),
            }
            self.counter += 1;
        }
    }
    impl<F> Scheduler<u32, u32, F> for MockScheduler<F>
    where
        F: Fn() -> (),
    {
        fn get_counter(&self) -> u32 {
            self.counter
        }
        fn finished(&self) -> bool {
            self.maybe_next_schedule.is_none()
        }
        fn schedule(&mut self, countdown: u32) {
            self.maybe_next_schedule = Some(self.counter + countdown);
        }
        fn schedule_at(&mut self, at: u32) {
            self.maybe_next_schedule = Some(at);
        }
        fn clear_interrupt(&mut self) {
            self.maybe_next_schedule = None;
        }
        fn set_callback(&mut self, f: F) {
            self.maybe_callback = Some(Box::new(f))
        }
    }
    // maybe_callback: Box<dyn Fn()> が Debug を derive できないので
    impl<F> std::fmt::Debug for MockScheduler<F>
    where
        F: Fn() -> (),
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockScheduler")
                .field("counter", &self.counter)
                .field("maybe_next_schedule", &self.maybe_next_schedule)
                .field("maybe_callback", &"Box<dyn Fn()>")
                .finish()
        }
    }

    // #[test]
    // fn test0() {
    //     let mut scheduler = MockScheduler::new();
    //     let mut led_pins = LedPins::init(
    //         MockLed::new(),
    //         MockLed::new(),
    //         MockLed::new(),
    //         MockLed::new(),
    //         &mut scheduler,
    //     );

    //     led_pins.scheduler().counter = 1;
    //     led_pins.set_led_mode(0, LedMode::HIGH);
    //     led_pins.set_mode_later(0, LedMode::LOW, 100);

    //     led_pins.scheduler().counter = 2;
    //     led_pins.set_led_mode(1, LedMode::HIGH);
    //     led_pins.set_mode_later(1, LedMode::LOW, 100);

    //     led_pins.scheduler().counter = 3;
    //     led_pins.set_led_mode(2, LedMode::HIGH);
    //     led_pins.set_mode_later(2, LedMode::LOW, 100);

    //     led_pins.scheduler().counter = 4;
    //     led_pins.set_led_mode(3, LedMode::HIGH);
    //     led_pins.set_mode_later(3, LedMode::LOW, 100);

    //     assert_eq!(
    //         led_pins.leds_status(),
    //         [
    //             LedStatus::HIGH,
    //             LedStatus::HIGH,
    //             LedStatus::HIGH,
    //             LedStatus::HIGH
    //         ]
    //     );

    //     led_pins.scheduler().counter = 101;
    //     led_pins.handle_schedule();
    //     assert_eq!(
    //         led_pins.leds_status(),
    //         [
    //             LedStatus::LOW,
    //             LedStatus::HIGH,
    //             LedStatus::HIGH,
    //             LedStatus::HIGH
    //         ]
    //     );

    //     led_pins.scheduler().counter = 102;
    //     led_pins.handle_schedule();
    //     assert_eq!(
    //         led_pins.leds_status(),
    //         [
    //             LedStatus::LOW,
    //             LedStatus::LOW,
    //             LedStatus::HIGH,
    //             LedStatus::HIGH
    //         ]
    //     );

    //     led_pins.scheduler().counter = 103;
    //     led_pins.handle_schedule();
    //     assert_eq!(
    //         led_pins.leds_status(),
    //         [
    //             LedStatus::LOW,
    //             LedStatus::LOW,
    //             LedStatus::LOW,
    //             LedStatus::HIGH
    //         ]
    //     );

    //     led_pins.scheduler().counter = 104;
    //     led_pins.handle_schedule();
    //     assert_eq!(
    //         led_pins.leds_status(),
    //         [
    //             LedStatus::LOW,
    //             LedStatus::LOW,
    //             LedStatus::LOW,
    //             LedStatus::LOW
    //         ]
    //     );
    // }

    #[test]
    fn test00() {
        let mut scheduler = MockScheduler::new();
        let mut led_pins = LedPins::init(
            MockLed::new(),
            MockLed::new(),
            MockLed::new(),
            MockLed::new(),
            &mut scheduler,
        );
        led_pins.scheduler.set_callback(|| println!(""));
        //                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ cyclic type of infinite size
        led_pins.scheduler.counter = 1;
        led_pins.set_led_mode(0, LedMode::HIGH);
        println!("{:#?}", led_pins);

        // led_pins.set_mode_later(0, LedMode::LOW, 100);
        // println!("{:#?}", led_pins);

        // led_pins.scheduler().counter = 2;
        // led_pins.set_led_mode(1, LedMode::HIGH);
        // led_pins.set_mode_later(1, LedMode::LOW, 100);

        // led_pins.scheduler().counter = 3;
        // led_pins.set_led_mode(2, LedMode::HIGH);
        // led_pins.set_mode_later(2, LedMode::LOW, 100);

        // led_pins.scheduler().counter = 4;
        // led_pins.set_led_mode(3, LedMode::HIGH);
        // led_pins.set_mode_later(3, LedMode::LOW, 100);

        // println!("{:#?}", led_pins);

        panic!();
    }

    // #[test]
    // fn test1() {
    //     let mut scheduler = MockScheduler::new();
    //     let mut led_pins: LedPins<u32, u32, MockLed, MockScheduler> = LedPins::init(
    //         MockLed::new(),
    //         MockLed::new(),
    //         MockLed::new(),
    //         MockLed::new(),
    //         &mut scheduler,
    //     );

    //     led_pins.set_led_mode(0, LedMode::BLINK);
    //     assert_eq!(
    //         led_pins.queue.peek(),
    //         Some(ScheduledPinsCommand {
    //             schedule: 250,
    //             led_num: 0,
    //             command: Command::ChangeLedStatus(LedStatus::HIGH) // FIXME 挙動変わった
    //         })
    //         .as_ref()
    //     );

    //     led_pins.handle_schedule();
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::LOW);

    //     led_pins.scheduler().counter = 250;
    //     led_pins.handle_schedule();
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::HIGH);

    //     led_pins.scheduler().counter = 499;
    //     led_pins.handle_schedule();
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::HIGH);

    //     led_pins.scheduler().counter = 500;
    //     led_pins.handle_schedule();
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::LOW);
    //     assert_eq!(
    //         led_pins.queue.peek(),
    //         Some(ScheduledPinsCommand {
    //             schedule: 750,
    //             led_num: 0,
    //             command: Command::ChangeLedStatus(LedStatus::HIGH)
    //         })
    //         .as_ref()
    //     );

    //     // BLINKモードからLOWモードに変える
    //     led_pins.scheduler().counter = 501;
    //     led_pins.set_led_mode(0, LedMode::LOW);
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::LOW);

    //     // BLINKモードの時のHIGHのスケジュールは無視され、LOWのままになる
    //     assert_eq!(
    //         led_pins.queue.peek(),
    //         Some(ScheduledPinsCommand {
    //             schedule: 750,
    //             led_num: 0,
    //             command: Command::ChangeLedStatus(LedStatus::HIGH)
    //         })
    //         .as_ref()
    //     );
    //     led_pins.scheduler().counter = 750;
    //     println!("{:#?}", led_pins);
    //     led_pins.handle_schedule();
    //     assert_eq!(led_pins.leds[0].get_status(), LedStatus::LOW);

    //     // todo!("some test yet implemented")
    // }
}
