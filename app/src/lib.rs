#![no_std]

mod write_to;

use crate::write_to::WriteTo;
use core::fmt::Write;

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

    /** 経過時間を64文字までのascii文字を返す */
    fn format_total_duration_long(counter: &Counter, app_elapsed_time: u128, is_active: bool) -> [u8; 64] {

        let millis = counter.get_total_duration(app_elapsed_time);

        const MS_PER_SEC: u128 = 1000;
        const MS_PER_MIN: u128 = MS_PER_SEC * 60;
        const MS_PER_HOUR: u128 = MS_PER_MIN * 60;
        const MS_PER_DAY: u128 = MS_PER_HOUR * 24;

        let days = millis / MS_PER_DAY;
        let hours = (millis % MS_PER_DAY) / MS_PER_HOUR;
        let minutes = (millis % MS_PER_HOUR) / MS_PER_MIN;
        let seconds = (millis % MS_PER_MIN) / MS_PER_SEC;
        let milliseconds = millis % MS_PER_SEC;

        let mut buffer: [u8; 64] = [0x20; 64];
        let mut w = WriteTo::new(&mut buffer);

        w.write_str(if is_active { "> " } else { "  " } ).unwrap();

        if days > 0 {
            w.write_fmt(format_args!("{} days, ", days)).unwrap()
        }
        if hours > 0 {
            w.write_fmt(format_args!("{} hours, ", hours)).unwrap()
        }
        if minutes > 0 {
            w.write_fmt(format_args!("{} minutes, ", minutes)).unwrap()
        }
        if seconds > 0 {
            w.write_fmt(format_args!("{} seconds, ", seconds)).unwrap()
        }
        w.write_fmt(format_args!("{} milliseconds", milliseconds)).unwrap();

        buffer
    }

    /** 4つのカウンターの内容を返す */
    pub fn format_total_durations_long(&self, app_elapsed_time: u128) -> [[u8; 64]; 4] {
        [
            Self::format_total_duration_long(&self.counters[0], app_elapsed_time, self.cursor == 0),
            Self::format_total_duration_long(&self.counters[1], app_elapsed_time, self.cursor == 1),
            Self::format_total_duration_long(&self.counters[2], app_elapsed_time, self.cursor == 2),
            Self::format_total_duration_long(&self.counters[3], app_elapsed_time, self.cursor == 3),
        ]
    }
}
