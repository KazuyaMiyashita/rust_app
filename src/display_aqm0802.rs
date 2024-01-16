use embedded_hal::blocking::i2c::Write;
use embedded_hal::blocking::delay::DelayMs;
use rp_pico as bsp;
use bsp::hal;
use hal::i2c::Error;
use hal::Timer;

const DISPLAY_I2C_ADDR: u8 = 0x3e;
const SETTING: u8 = 0x00;
const DISPLAY: u8 = 0x40;
const INIT_1: [u8; 6] = [0x38, 0x39, 0x14, 0x73, 0x56, 0x6c];
const INIT_2: [u8; 3] = [0x38, 0x0c, 0x01];
// cf. [Raspberry Pi Picoで液晶ディスプレイモジュール (AQM0802A+ PCA9515)を使う方法](https://info.picaca.jp/15213)

pub struct DisplayAQM0802<I2C: Write> {
    i2c: I2C,
    timer: Timer,
}

impl<I2C> DisplayAQM0802<I2C>
    where I2C: Write,
          Error: From<<I2C as Write>::Error>
{
    pub fn init_blocking(mut i2c: I2C, timer: &mut Timer) -> Result<Self, Error> {
        timer.delay_ms(40);

        for op in INIT_1 {
            i2c.write(DISPLAY_I2C_ADDR, &[SETTING])?;
            timer.delay_ms(1);
            i2c.write(DISPLAY_I2C_ADDR, &[op])?;
            timer.delay_ms(1)
        }
        timer.delay_ms(200);
        for op in INIT_2 {
            i2c.write(DISPLAY_I2C_ADDR, &[SETTING])?;
            timer.delay_ms(1);
            i2c.write(DISPLAY_I2C_ADDR, &[op])?;
            timer.delay_ms(1)
        }
        timer.delay_ms(200);

        Ok(DisplayAQM0802 { i2c, timer: *timer })
    }

    pub fn print_blocking(&mut self, message: &str) -> Result<(), Error> {
        self.i2c.write(DISPLAY_I2C_ADDR, &[SETTING])?;
        self.timer.delay_ms(1);
        self.i2c.write(DISPLAY_I2C_ADDR, &[0x01])?;
        self.timer.delay_ms(1);

        for op in message.as_bytes() {
            self.i2c.write(DISPLAY_I2C_ADDR, &[DISPLAY])?;
            self.timer.delay_ms(1);
            self.i2c.write(DISPLAY_I2C_ADDR, &[op.clone()])?;
            self.timer.delay_ms(1)
        }

        Ok(())
    }
}

