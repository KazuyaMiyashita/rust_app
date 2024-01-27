use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::Write;

use rp_pico as bsp;

use bsp::hal::i2c::Error;
use bsp::hal::Timer;
use defmt::debug;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayUs;

const DISPLAY_I2C_ADDR: u8 = 0x3e;
const SETTING: u8 = 0x00;
const DISPLAY: u8 = 0x40;
const INIT_1: [u8; 6] = [
    // 0x38 と 0x39 はどちらか片方だけでも良さそう？未調査。
    // Function set. (?)
    // 8-bit bus mode with MPU, 2-line display mode, Double height font is forbidden. (normal instruction be selected)
    0x38, // Wait time > 26.3 μs
    // Function set. (?)
    // 8-bit bus mode with MPU, 2-line display mode, Double height font is forbidden. (extension instruction be selected)
    0x39, // Wait time > 26.3 μs
    // (データシートには Internal OSC frequency と説明があるが、 Cursor or Display Shift のことだろう)
    // Cursor is controlled by R/L bit, set direction to right
    0x14, // Wait time > 26.3 μs
    // Contrast set (0x70)
    // 下位4ビットでコントラストを調整できる。0x7eだと真っ黒
    0x73, // Wait time > 26.3 μs
    // Power/ICON/Contrast control
    // 下から4bit目を1にすると文字の背景をやや暗くしなくなる。
    0x56, // Wait time > 26.3 μs
    // Follower control (?)
    0x6c, // Wait time > 200 ms
];
const INIT_2: [u8; 3] = [
    // Function set (?)
    0x38, // Wait time > 26.3 μs
    // Display ON/OFF control
    0x0c, // Wait time > 26.3 μs
    // Clear Display
    0x01, // Wait time > 1.08 ms
];
// cf. [Raspberry Pi Picoで液晶ディスプレイモジュール (AQM0802A+ PCA9515)を使う方法](https://info.picaca.jp/15213)

pub struct DisplayAQM0802<I2C: Write> {
    i2c: I2C,
    timer: Timer,
}

impl<I2C> DisplayAQM0802<I2C>
where
    I2C: Write,
    Error: From<<I2C as Write>::Error>,
{
    pub fn init_blocking(mut i2c: I2C, timer: &mut Timer) -> Result<Self, Error> {
        // なぜか一度スキャンしないと動かない
        let mut found = false;
        for address in 0..=127 {
            match i2c.write(address, &[1]) {
                Ok(_) => {
                    debug!("Found device on address {}\n", address);
                    if address == DISPLAY_I2C_ADDR {
                        found = true
                    }
                }
                Err(_) => {}
            }
        }
        if !found {
            panic!("AQM0802 not found") // FIXME
        }

        timer.delay_ms(40); // Wait time > 40ms After VDD stable

        for op in INIT_1 {
            i2c.write(DISPLAY_I2C_ADDR, &[SETTING, op])?;
            timer.delay_us(27);
        }
        timer.delay_ms(200);
        for op in INIT_2 {
            i2c.write(DISPLAY_I2C_ADDR, &[SETTING, op])?;
            timer.delay_us(27);
        }
        timer.delay_us(1080);

        Ok(DisplayAQM0802 { i2c, timer: *timer })
    }

    // 最大8文字*2行の文字列を表示する。
    // 9バイト目以降は2列目に表示される。16バイト未満が指定された時は空白文字(0x20)で埋められ、17バイト目以降は時は無視される。
    #[allow(dead_code)]
    pub fn print_blocking(&mut self, message: &[u8]) -> Result<(), Error> {
        let mut line0: [u8; 8] = [0x20; 8];
        let mut line1: [u8; 8] = [0x20; 8];

        for (index, &byte) in message.iter().enumerate().take(16) {
            if index < 8 {
                line0[index] = byte;
            } else {
                line1[index - 8] = byte;
            }
        }

        self.print_blocking2(&line0, &line1)
    }

    // 最大8文字*2行の文字列を表示する。2行それぞれ指定するバージョン。
    pub fn print_blocking2(&mut self, line0: &[u8], line1: &[u8]) -> Result<(), Error> {
        self.clear_display()?;

        for b in line0.iter().take(8) {
            self.i2c.write(DISPLAY_I2C_ADDR, &[DISPLAY, b.clone()])?;
        }

        // 2行目の先頭へ(Set DDDRAM address. Wait time > 26.3 μs)
        self.i2c.write(DISPLAY_I2C_ADDR, &[SETTING, 0xC0])?;
        self.timer.delay_us(27);

        for b in line1.iter().take(8) {
            self.i2c.write(DISPLAY_I2C_ADDR, &[DISPLAY, b.clone()])?;
        }

        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<(), Error> {
        self.i2c.write(DISPLAY_I2C_ADDR, &[SETTING, 0x01])?; // Clear Display
        self.timer.delay_us(1081);

        Ok(())
    }
}
