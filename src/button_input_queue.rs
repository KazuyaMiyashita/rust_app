/// 割り込みを利用してボタンの入力をキューにためる
use defmt::{debug, info, warn, Format};

use alloc::vec::Vec;
use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, gpio::Interrupt::EdgeHigh, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::InputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm0, Instant};
use rp_pico::hal::Timer;

type Button0Pin = gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSioInput, gpio::PullDown>;
type Button1Pin = gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSioInput, gpio::PullDown>;
type Button2Pin = gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSioInput, gpio::PullDown>;
type Button3Pin = gpio::Pin<gpio::bank0::Gpio16, gpio::FunctionSioInput, gpio::PullDown>;

struct ButtonPins {
    button0: Button0Pin,
    button1: Button1Pin,
    button2: Button2Pin,
    button3: Button3Pin,
}

#[derive(Copy, Clone, PartialEq, Format)]
pub enum ButtonInput {
    Button0,
    Button1,
    Button2,
    Button3,
}

// ボタン押下のGPIOの割り込みが来た後、10ms後にタイマーをセットしそのボタンがチャタリングではなく正しく押されているかを確認するためのデータの置き場所
// 一度ボタンが押下された後、タイマーによる処理が行われるまでは同じボタンは登録しない
// 複数のボタンが押下された時、それぞれのボタンが押されてから10ms後にタイマーをセットしたいが、タイマーには複数セットすることができないため
// 一つの値をセットした後、次の値はバッファーに入れて最初の値のタイマー処理後に次のタイマーをセットする。
struct ButtonInterrupts {
    button0: Option<Instant>,
    button1: Option<Instant>,
    button2: Option<Instant>,
    button3: Option<Instant>,
    timer: Timer,
    alarm: Alarm0,
}

const BUTTON_INPUT_QUEUE_LENGTH: usize = 20;
pub struct ButtonInputQueue {
    buffer: [ButtonInput; BUTTON_INPUT_QUEUE_LENGTH],
    cursor: usize, // 次にバッファに書き込む位置。
}

static GLOBAL_BUTTON_PINS: Mutex<RefCell<Option<ButtonPins>>> = Mutex::new(RefCell::new(None));

static GLOBAL_BUTTON_INTERRUPTS: Mutex<RefCell<Option<ButtonInterrupts>>> =
    Mutex::new(RefCell::new(None));

static GLOBAL_BUTTON_INPUT_QUEUE: Mutex<RefCell<Option<ButtonInputQueue>>> =
    Mutex::new(RefCell::new(None));

impl ButtonInterrupts {
    fn new(timer: Timer, alarm: Alarm0) -> Self {
        ButtonInterrupts {
            button0: None,
            button1: None,
            button2: None,
            button3: None,
            timer,
            alarm,
        }
    }

    // 全てのボタンがまだ押されていない時に限り
    // TODO
    fn add(&mut self, button_input: ButtonInput) {
        match button_input {
            ButtonInput::Button0 => match self.button0 {
                None => {
                    self.button0 = Some(self.timer.get_counter())
                }
                Some(instant) =>
            }
        }

    }

    // 最も直近の
    // TODO
    fn get_event_and_set_next(&mut self) -> Option<ButtonInput> {
        // let maybeCurrent = self.buffer[self.cursor];
        // match maybeCurrent {
        //     None => {
        //         warn!("current is empty, but get_event_and_set_next called. why?");
        //         ()
        //     }
        //     Some(current) => {
        //         let maybeNext = self. // TODO 続きを
        //     }
        // }
        todo!()
    }
}

impl ButtonInputQueue {
    fn new() -> Self {
        ButtonInputQueue {
            buffer: [ButtonInput::Button0; BUTTON_INPUT_QUEUE_LENGTH],
            cursor: 0,
        }
    }

    pub fn init(
        button0: Button0Pin,
        button1: Button1Pin,
        button2: Button2Pin,
        button3: Button3Pin,
        timer: Timer,
        mut alarm: Alarm0,
    ) {
        button0.set_interrupt_enabled(EdgeHigh, true);
        button1.set_interrupt_enabled(EdgeHigh, true);
        button2.set_interrupt_enabled(EdgeHigh, true);
        button3.set_interrupt_enabled(EdgeHigh, true);
        alarm.enable_interrupt();

        // Give away our pins by moving them into the `GLOBAL_PINS` variable.
        // We won't need to access them in the main thread again
        critical_section::with(|cs| {
            GLOBAL_BUTTON_PINS.borrow(cs).replace(Some(ButtonPins {
                button0,
                button1,
                button2,
                button3,
            }));

            GLOBAL_BUTTON_INTERRUPTS
                .borrow(cs)
                .replace(Some(ButtonInterrupts::new(timer, alarm)));

            GLOBAL_BUTTON_INPUT_QUEUE
                .borrow(cs)
                .replace(Some(ButtonInputQueue::new()));
        });

        // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
        // will jump to the interrupt function when the interrupt occurs.
        // We do this last so that the interrupt can't go off while
        // it is in the middle of being configured
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        }
    }

    pub fn pop_all() -> Vec<ButtonInput> {
        critical_section::with(|cs| {
            match GLOBAL_BUTTON_INPUT_QUEUE.borrow(cs).borrow_mut().as_mut() {
                None => {
                    warn!("GLOBAL_BUTTON_PRESSED_QUEUE not initialized. why?");
                    Vec::new()
                }
                Some(q) => {
                    let vec = Vec::from(&mut q.buffer[0..q.cursor]);
                    q.cursor = 0;
                    vec
                }
            }
        })
    }

    // 追加できたら Some(()), バッファが足りなかくて追加できなかったら None
    fn push(&mut self, v: ButtonInput) -> Option<()> {
        if self.cursor <= BUTTON_INPUT_QUEUE_LENGTH {
            self.buffer[self.cursor] = v;
            self.cursor += 1;
            Some(())
        } else {
            None
        }
    }
}

/// ボタンの割り込みがきた時、どのボタンによる割り込みかを判断し、ボタンの割り込みをクリアする。
/// その後、10ms後に再度判断を行うためにタイマーの割り込みのキューに追加する
#[interrupt]
fn IO_IRQ_BANK0() {
    critical_section::with(|cs| {
        debug!("IO_IRQ_BANK0");

        let mut button_pins_binding = GLOBAL_BUTTON_PINS.borrow(cs).borrow_mut();
        let button_pins = button_pins_binding.as_mut().unwrap();

        let mut button_interrupts_binding = GLOBAL_BUTTON_INTERRUPTS.borrow(cs).borrow_mut();
        let button_interrupts = button_interrupts_binding.as_mut().unwrap();

        let innterrupted_buttons: [Option<ButtonInput>; 4] = [
            if button_pins.button0.interrupt_status(EdgeHigh) {
                button_pins.button0.clear_interrupt(EdgeHigh);
                Some(ButtonInput::Button0)
            } else {
                None
            },
            if button_pins.button1.interrupt_status(EdgeHigh) {
                button_pins.button1.clear_interrupt(EdgeHigh);
                Some(ButtonInput::Button1)
            } else {
                None
            },
            if button_pins.button2.interrupt_status(EdgeHigh) {
                button_pins.button2.clear_interrupt(EdgeHigh);
                Some(ButtonInput::Button2)
            } else {
                None
            },
            if button_pins.button3.interrupt_status(EdgeHigh) {
                button_pins.button3.clear_interrupt(EdgeHigh);
                Some(ButtonInput::Button3)
            } else {
                None
            },
        ];

        for maybe_button in innterrupted_buttons.iter() {
            if let Some(button) = maybe_button {
                debug!("{} EdgeHigh", button);
                button_interrupts.add(*button)
            }
        }
    })
}

/// タイマーの割り込みが来た時、どのボタンによってタイマーが登録されたかを判断し、タイマーの割り込みのクリアおよび次のタイマーの設定を行う。
/// その後、そのボタンが正しく押され続けてたかを判断し、押されていたら押されたボタンキューに追加する
#[interrupt]
fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        debug!("TIMER_IRQ_0");

        let mut button_pins_binding = GLOBAL_BUTTON_PINS.borrow(cs).borrow_mut();
        let button_pins = button_pins_binding.as_mut().unwrap();

        let mut button_interrupts_binding = GLOBAL_BUTTON_INTERRUPTS.borrow(cs).borrow_mut();
        let button_interrupts = button_interrupts_binding.as_mut().unwrap();

        let mut button_input_queue_binding = GLOBAL_BUTTON_INPUT_QUEUE.borrow(cs).borrow_mut();
        let button_input_queue = button_input_queue_binding.as_mut().unwrap();

        let maybe_button_event = button_interrupts.get_event_and_set_next();
        if let Some(button_event) = maybe_button_event {
            match button_event {
                ButtonInput::Button0 => {
                    if button_pins.button0.is_high().unwrap() {
                        button_input_queue.push(ButtonInput::Button0);
                    }
                }
                ButtonInput::Button1 => {
                    if button_pins.button1.is_high().unwrap() {
                        button_input_queue.push(ButtonInput::Button1);
                    }
                }
                ButtonInput::Button2 => {
                    if button_pins.button2.is_high().unwrap() {
                        button_input_queue.push(ButtonInput::Button2);
                    }
                }
                ButtonInput::Button3 => {
                    if button_pins.button3.is_high().unwrap() {
                        button_input_queue.push(ButtonInput::Button3);
                    }
                }
            }
        }
    })
}
