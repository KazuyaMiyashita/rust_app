#![no_std]

pub enum Key {
    AllowUp,
    AllowDown,
    AllowLeft,
    AllowRight,
}

pub struct App {
    counters: [Counter; 4],
    pub cursor: usize,
}

#[derive(Copy, Clone)]
struct Counter {
    past_elapsed_duration: u128,
    last_activated_time: Option<u128>,
}

impl Counter {
    fn init() -> Counter {
        Counter {
            past_elapsed_duration: 0,
            last_activated_time: None,
        }
    }

    fn activate(&mut self, app_elapsed_time: u128) -> () {
        match self.last_activated_time {
            None => self.last_activated_time = Some(app_elapsed_time),
            Some(_) => ()
        }
    }

    pub fn deactivate(&mut self, app_elapsed_time: u128) -> () {
        match self.last_activated_time {
            None => (),
            Some(t) => {
                self.past_elapsed_duration += app_elapsed_time - t;
                self.last_activated_time = None;
            }
        }
    }

    fn get_total_duration(&self, app_elapsed_time: u128) -> u128 {
        match self.last_activated_time {
            None => self.past_elapsed_duration,
            Some(t) => self.past_elapsed_duration + (app_elapsed_time - t)
        }
    }
}

impl App {
    pub fn init() -> App {
        App {
            counters: [Counter::init(); 4],
            cursor: 0,
        }
    }

    pub fn on_key_down(&mut self, key: Key, app_elapsed_time: u128) -> () {
        match key {
            Key::AllowUp => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            Key::AllowDown => {
                if self.cursor < 3 {
                    self.cursor += 1;
                }
            }
            Key::AllowLeft => {
                self.counters[self.cursor].deactivate(app_elapsed_time);
            }
            Key::AllowRight => {
                self.counters[self.cursor].activate(app_elapsed_time);
            }
        }
    }


    pub fn elapsed_times_ms(&self, app_elapsed_time: u128) -> [u128; 4] {
        [
            self.counters[0].get_total_duration(app_elapsed_time),
            self.counters[1].get_total_duration(app_elapsed_time),
            self.counters[2].get_total_duration(app_elapsed_time),
            self.counters[3].get_total_duration(app_elapsed_time)
        ]
    }
}
