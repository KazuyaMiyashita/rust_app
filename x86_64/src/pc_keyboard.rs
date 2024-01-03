use pc_keyboard::{DecodedKey, KeyState, HandleControl, ScancodeSet1};
use pc_keyboard::layouts::Us104Key;
use x86_64::instructions::port::Port;

pub struct PCKeyboard {
    port: Port<u8>,
    kbd: pc_keyboard::Keyboard<Us104Key, ScancodeSet1>,
    pressed: bool,
}

impl PCKeyboard {
    pub fn init() -> PCKeyboard {
        PCKeyboard {
            port: Port::new(0x60),
            kbd: pc_keyboard::Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::MapLettersToUnicode),
            pressed: false
        }
    }

    pub fn poll_char(&mut self) -> Option<char> {
        let b: u8 = unsafe { self.port.read() };
        let event = self.kbd.add_byte(b).ok()??;

        match (event.state, self.pressed) {
            (KeyState::Down, true) => None,
            (KeyState::Down, false) => {
                self.pressed = true;
                match self.kbd.process_keyevent(event) {
                    Some(DecodedKey::Unicode(c)) => Some(c),
                    _ => None,
                }
            }
            (KeyState::Up, true) => {
                self.pressed = false;
                None
            }
            (KeyState::Up, false) => None,
            (KeyState::SingleShot, _) => None,
        }
    }

    #[allow(unused)]
    pub fn get_char_blocking(&mut self) -> char {
        loop {
            if let Some(ch) = self.poll_char() {
                return ch;
            }
        }
    }
}

