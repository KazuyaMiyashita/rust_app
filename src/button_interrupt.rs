use defmt::{debug, info, warn};

use alloc::vec::Vec;
use bsp::hal::fugit::ExtU32;
use bsp::hal::{gpio, gpio::Interrupt::EdgeHigh, pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal::digital::v2::InputPin;
use fugit::MicrosDurationU32;
use rp_pico as bsp;
use rp_pico::hal::Timer;
use rp_pico::hal::timer::{Alarm, Alarm0, Instant};

/// This how we transfer our Led and Button pins into the Interrupt Handler.
/// We'll have the option hold both using the LedAndButton type.
/// This will make it a bit easier to unpack them later.
static GLOBAL_KEY_INTERRUPT_COMPONENT: Mutex<RefCell<Option<KeyInterruptComponent>>> =
    Mutex::new(RefCell::new(None));

static GLOBAL_BUTTON_PRESSED_QUEUE: Mutex<RefCell<Option<FixedSizeQueue<ButtonInput, 20>>>> =
    Mutex::new(RefCell::new(None));

static GLOBAL_FIXED_TIME_SCHEDULER: Mutex<RefCell<Option<ButtonEventQueue<ButtonInput, 4>>>> =
    Mutex::new(RefCell::new(None));

struct FixedSizeQueue<T, const N: usize> {
    buffer: [T; N],
    cursor: usize, // 次にバッファに書き込む位置。
}

impl<T: Copy + Clone, const N: usize> FixedSizeQueue<T, N> {
    fn new(any_value: T) -> Self {
        FixedSizeQueue {
            buffer: [any_value; N],
            cursor: 0,
        }
    }

    fn pop_all(&mut self) -> Vec<T> {
        let vec = Vec::from(&mut self.buffer[0..self.cursor]);
        self.cursor = 0;
        vec
    }

    // 追加できたら Some(()), バッファが足りなかくて追加できなかったら None
    fn push(&mut self, v: T) -> Option<()> {
        if self.cursor <= N {
            self.buffer[self.cursor] = v;
            self.cursor += 1;
            Some(())
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ButtonInput {
    Button0,
    Button1,
    Button2,
    Button3,
}

/// This pin will be our interrupt source.
/// ~~It will trigger an interrupt if pulled to ground (via a switch or jumper wire)~~
/// 電流が流れている時にONとしたいので、PullUp から PullDown に変更した
type Button0Pin = gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSioInput, gpio::PullDown>;
type Button1Pin = gpio::Pin<gpio::bank0::Gpio18, gpio::FunctionSioInput, gpio::PullDown>;
type Button2Pin = gpio::Pin<gpio::bank0::Gpio17, gpio::FunctionSioInput, gpio::PullDown>;
type Button3Pin = gpio::Pin<gpio::bank0::Gpio16, gpio::FunctionSioInput, gpio::PullDown>;

/// Since we're always accessing these pins together we'll store them in a tuple.
/// Giving this tuple a type alias means we won't need to use () when putting them
/// inside an Option. That will be easier to read.
struct KeyInterruptComponent {
    button0: Button0Pin,
    button1: Button1Pin,
    button2: Button2Pin,
    button3: Button3Pin,
    alarm: Alarm0,
    button0_has_edge_high: bool,
    button1_has_edge_high: bool,
    button2_has_edge_high: bool,
    button3_has_edge_high: bool,
}

// ボタン押下のGPIOの割り込みが来た後、10ms後にタイマーをセットしそのボタンがチャタリングではなく正しく押されているかを確認するためのキュー
// 一度ボタンが押下された後、タイマーによる処理が行われるまでは同じボタンは登録しない
// 複数のボタンが押下された時、それぞれのボタンが押されてから10ms後にタイマーをセットしたいが、タイマーには複数セットすることができないため
// 一つの値をセットした後、次の値はバッファーに入れて最初の値のタイマー処理後に次のタイマーをセットする。
struct ButtonEventQueue<const N: usize> {
    buffer: [Option<(ButtonInput, Instant)>; N],
    cursor: usize, // 次にバッファに書き込む位置。 0 ~ n-1。
    timer: Timer,
    alarm: Alarm0,
}

impl <const N: usize>ButtonEventQueue<N> {
    fn new(timer: Timer, alarm: Alarm0) -> Self {
        ButtonEventQueue {
            buffer: [None; N],
            cursor: 0,
            timer,
            alarm,
        }
    }

    fn add(&mut self, event_type: ButtonInput) {
        if self.buffer.iter().any(|v| v.0 == event_type) {
            () // もしその event_type のものがすでに登録されていたら何もしない
        } else {
            self.buffer[self.cursor] = Some((event_type, self.timer.get_counter()));
            self.cursor = (self.cursor + 1) % N;
        }
    }

    fn get_event_and_set_next(&mut self) -> Option<ButtonInput> {

        let maybeCurrent = self.buffer[self.cursor];
        match maybeCurrent {
            None => {
                warn!("current is empty, but get_event_and_set_next called. why?");
                ()
            }
            Some(current) => {
                let maybeNext = self. // TODO 続きを
            }
        }

    }
}

pub struct ButtonInterrupt {}
impl ButtonInterrupt {
    pub fn enable_interrupt(
        button0: Button0Pin,
        button1: Button1Pin,
        button2: Button2Pin,
        button3: Button3Pin,
        mut alarm: Alarm0,
    ) -> Self {
        button0.set_interrupt_enabled(EdgeHigh, true);
        button1.set_interrupt_enabled(EdgeHigh, true);
        button2.set_interrupt_enabled(EdgeHigh, true);
        button3.set_interrupt_enabled(EdgeHigh, true);
        alarm.enable_interrupt();

        // Give away our pins by moving them into the `GLOBAL_PINS` variable.
        // We won't need to access them in the main thread again
        critical_section::with(|cs| {
            GLOBAL_BUTTON_PRESSED_QUEUE
                .borrow(cs)
                .replace(Some(FixedSizeQueue::new(ButtonInput::Button0)));

            GLOBAL_KEY_INTERRUPT_COMPONENT
                .borrow(cs)
                .replace(Some(KeyInterruptComponent {
                    button0,
                    button1,
                    button2,
                    button3,
                    alarm,
                    button0_has_edge_high: false,
                    button1_has_edge_high: false,
                    button2_has_edge_high: false,
                    button3_has_edge_high: false,
                }))
        });

        // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
        // will jump to the interrupt function when the interrupt occurs.
        // We do this last so that the interrupt can't go off while
        // it is in the middle of being configured
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0);
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0);
        }

        ButtonInterrupt {}
    }

    pub fn pop_button_inputs(&self) -> Vec<ButtonInput> {
        critical_section::with(|cs| {
            match GLOBAL_BUTTON_PRESSED_QUEUE.borrow(cs).borrow_mut().as_mut() {
                None => {
                    warn!("GLOBAL_BUTTON_PRESSED_QUEUE not initialized. why?");
                    Vec::new()
                }
                Some(q) => q.pop_all(),
            }
        })
    }
}

#[interrupt]
fn IO_IRQ_BANK0() {
    critical_section::with(|cs| {
        debug!("IO_IRQ_BANK0");

        let mut binding = GLOBAL_KEY_INTERRUPT_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();
        // Check if the interrupt source is from the push button going from high-to-low.
        // Note: this will always be true in this example, as that is the only enabled GPIO interrupt source
        if component.button0.interrupt_status(EdgeHigh) {
            debug!("button0 EdgeHigh");
            component.button0.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button0_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }

        if component.button1.interrupt_status(EdgeHigh) {
            debug!("button1 EdgeHigh");
            component.button1.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button1_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }

        if component.button2.interrupt_status(EdgeHigh) {
            debug!("button2 EdgeHigh");
            component.button2.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button2_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }

        if component.button3.interrupt_status(EdgeHigh) {
            debug!("button3 EdgeHigh");
            component.button3.clear_interrupt(EdgeHigh);

            // すでにスケジュールが動いている時は上書きしない
            component.button3_has_edge_high = true;
            if component.alarm.finished() {
                component.alarm.schedule(10.millis()).unwrap();
            }
        }
    })
}

#[interrupt]
fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        debug!("TIMER_IRQ_0");

        let mut binding = GLOBAL_KEY_INTERRUPT_COMPONENT.borrow(cs).borrow_mut();
        let component = binding.as_mut().unwrap();

        if component.button0_has_edge_high {
            if component.button0.is_high().unwrap() {
                info!("BUTTON0 PRESSED!");
                match GLOBAL_BUTTON_PRESSED_QUEUE.borrow(cs).borrow_mut().as_mut() {
                    None => warn!("GLOBAL_BUTTON_PRESSED_QUEUE not initialized. why?"),
                    Some(q) => q
                        .push(ButtonInput::Button0)
                        .unwrap_or_else(|| info!("GLOBAL_BUTTON_PRESSED_QUEUE limit exceeded.")),
                }
            }
            component.button0_has_edge_high = false;
        }
        if component.button1_has_edge_high {
            if component.button1.is_high().unwrap() {
                info!("BUTTON1 PRESSED!");
                match GLOBAL_BUTTON_PRESSED_QUEUE.borrow(cs).borrow_mut().as_mut() {
                    None => warn!("GLOBAL_BUTTON_PRESSED_QUEUE not initialized. why?"),
                    Some(q) => q
                        .push(ButtonInput::Button1)
                        .unwrap_or_else(|| info!("GLOBAL_BUTTON_PRESSED_QUEUE limit exceeded.")),
                }
            }
            component.button1_has_edge_high = false;
        }

        component.alarm.clear_interrupt()
    })
}
