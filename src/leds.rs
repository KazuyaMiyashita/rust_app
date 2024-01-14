use core::convert::Infallible;
use rp2040_hal as hal;
use hal::gpio::{Pin, DynPinId, FunctionSio, SioOutput, PullDown};
use embedded_hal::digital::v2::OutputPin;

pub struct LEDs {
    pub _0: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
    pub _1: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
    pub _2: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
    pub _3: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
}

impl LEDs {
    pub fn new(
        _0: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
        _1: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
        _2: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
        _3: Pin<DynPinId, FunctionSio<SioOutput>, PullDown>,
    ) -> Self {
        LEDs {
            _0,
            _1,
            _2,
            _3,
        }
    }

    pub fn light(&mut self, n: u8) -> Result<(), Infallible> {

        for (i, pin) in [
            &mut self._0,
            &mut self._1,
            &mut self._2,
            &mut self._3,
        ]
            .iter_mut()
            .enumerate()
        {
            if &n & (1 << i) != 0 {
                pin.set_high()?;
            } else {
                pin.set_low()?;
            }
        }

        Ok(())
    }

}