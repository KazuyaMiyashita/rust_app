use embedded_hal::blocking::i2c::Write;

use rp_pico as bsp;

use crate::display_aqm0802::DisplayAQM0802;
use bsp::hal::i2c::Error;
use bsp::hal::Timer;
use core::fmt;

pub struct Console<I2C: Write> {
    display: DisplayAQM0802<I2C>,
    buffer: [[u8; 8]; 2],
    cursor: usize,
    next_new_line: bool,
}

impl<I2C> Console<I2C>
where
    I2C: Write,
    Error: From<<I2C as Write>::Error>,
{
    pub fn init_blocking(i2c: I2C, timer: &mut Timer) -> Result<Self, Error> {
        let display = DisplayAQM0802::init_blocking(i2c, timer)?;
        let buffer = [[0x20u8; 8]; 2];
        Ok(Console {
            display,
            buffer,
            cursor: 0,
            next_new_line: false,
        })
    }

    fn add_char(&mut self, c: &u8) -> () {
        if self.next_new_line {
            self.buffer[1] = self.buffer[0].clone();
            self.buffer[0] = [0x20; 8];
            self.cursor = 0;
            self.next_new_line = false;
        }
        if self.cursor < 8 {
            self.buffer[0][self.cursor.clone()] = c.clone();
            self.cursor += 1;
        }
    }
    fn print(&mut self) -> Result<(), Error> {
        self.display
            .print_blocking2(&self.buffer[1], &self.buffer[0])
    }
}

impl<I2C> fmt::Write for Console<I2C>
where
    I2C: Write,
    Error: From<<I2C as Write>::Error>,
{
    fn write_str(&mut self, str: &str) -> fmt::Result {
        for c in str.as_bytes() {
            if *c == b'\n' {
                // ディスプレイが2行しかないので、改行文字がきた時に
                // すぐに改行するのではなく、次に文字がきた時に改行する
                self.next_new_line = true;
            } else {
                self.add_char(c)
            }
        }

        self.print().map_err(|_| fmt::Error)
    }
}
