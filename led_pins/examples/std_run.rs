use led_pins::{Duration, Instant, Led, LedMode, LedPins, LedStatus, Scheduler};
use std::ops::Add;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct MyInstant(std::time::Instant);
impl Instant for MyInstant {
    fn add_millis(&self, millis: u32) -> Self {
        MyInstant(self.0.add(std::time::Duration::from_millis(millis.into())))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Debug)]
struct MyDuration(std::time::Duration);

impl Duration for MyDuration {
    fn from_millis(millis: u32) -> Self {
        MyDuration(std::time::Duration::from_millis(millis.into()))
    }
    fn to_millis(&self) -> u32 {
        self.0.as_millis() as u32
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
    fn set_status(&mut self, led_status: LedStatus) {
        self.led_status = led_status;
    }
}

#[derive(Debug)]
struct MockScheduler {
    maybe_next_schedule: Option<MyInstant>,
}
impl MockScheduler {
    fn new() -> Self {
        MockScheduler {
            maybe_next_schedule: None,
        }
    }
}
impl Scheduler<MyInstant, MyDuration> for MockScheduler {
    fn get_counter(&self) -> MyInstant {
        MyInstant(std::time::Instant::now())
    }
    fn finished(&self) -> bool {
        self.maybe_next_schedule.is_none()
    }
    fn schedule(&mut self, countdown: MyDuration) {
        self.maybe_next_schedule = Some(MyInstant(self.get_counter().0.add(countdown.0)));
    }
    fn schedule_at(&mut self, at: MyInstant) {
        self.maybe_next_schedule = Some(at);
    }
    fn clear_interrupt(&mut self) {
        self.maybe_next_schedule = None;
    }
}

use std::io;
use std::io::Write;

fn main() {
    let mut led_pins = LedPins::init(
        MockLed::new(),
        MockLed::new(),
        MockLed::new(),
        MockLed::new(),
        MockScheduler::new(),
    );

    loop {
        println!("{:#?}", led_pins);

        println!("input some command or quit to input `quit`");
        print!("> ");
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read line.");
        let input = buffer.trim().to_string();

        match input.as_str() {
            "quit" => break,
            "q" => led_pins.set_led_mode(0, LedMode::LOW),
            "w" => led_pins.set_led_mode(0, LedMode::LOW),
            "r" => led_pins.set_led_mode(0, LedMode::LOW),
            "t" => led_pins.set_led_mode(0, LedMode::LOW),
            "a" => led_pins.set_led_mode(0, LedMode::HIGH),
            "s" => led_pins.set_led_mode(0, LedMode::HIGH),
            "d" => led_pins.set_led_mode(0, LedMode::HIGH),
            "f" => led_pins.set_led_mode(0, LedMode::HIGH),
            "z" => led_pins.set_led_mode(0, LedMode::BLINK),
            "x" => led_pins.set_led_mode(0, LedMode::BLINK),
            "c" => led_pins.set_led_mode(0, LedMode::BLINK),
            "v" => led_pins.set_led_mode(0, LedMode::BLINK),
            _ => println!("continue"),
        }
    }
}
