use rp2040_hal as hal;
use hal::gpio::{Pins, Pin, FunctionSio, SioOutput, PullDown};
use hal::gpio::bank0::{Gpio10, Gpio11, Gpio12, Gpio13};
use embedded_hal::digital::v2::{InputPin, OutputPin};
use core::convert::Infallible;

pub struct LEDs<T> {
    pub _0: T,
    pub _1: T,
    pub _2: T,
    pub _3: T,
}

impl <T> LEDs<T>
where T: OutputPin<Error=Infallible>{
    pub fn new(
        _0: T,
        _1: T,
        _2: T,
        _3: T,
    ) -> Self {
        LEDs {
            _0,
            _1,
            _2,
            _3,
        }
    }

    pub fn light(&mut self, n: u8) -> () {

        let arr = [&self._0, &self._1, &self._2, &self._3];
        for i in 0..4 {
            if &n & (1 << &i) != 0 {
                arr[&i].set_low().unwrap();
            } else {
                arr[&i].set_high().unwrap();
            }
        }

    }

}