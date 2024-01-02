pub enum Key {
    AllowUp,
    AllowDown,
    AllowLeft,
    AllowRight,
}

pub struct App {
    nums: [u8; 4],
    cursor: usize,
}

impl App {
    pub fn init() -> App {
        App {
            nums: [0; 4],
            cursor: 0,
        }
    }

    pub fn on_key_down(&mut self, key: Key) -> () {
        match key {
            Key::AllowUp => {
                if self.nums[self.cursor] < 255 {
                    self.nums[self.cursor] += 1;
                }
            }
            Key::AllowDown => {
                if self.nums[self.cursor] > 0 {
                    self.nums[self.cursor] -= 1;
                }
            }
            Key::AllowLeft => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            Key::AllowRight => {
                if self.cursor < 3 {
                    self.cursor += 1;
                }
            }
        }
    }

    pub fn to_string(&self) -> String {
        let pad = " ".repeat(3 + self.cursor * 5);
        format!("[{:03}, {:03}, {:03}, {:03}]\n{}^",
                self.nums[0],
                self.nums[1],
                self.nums[2],
                self.nums[3],
                pad
        )
    }
}
