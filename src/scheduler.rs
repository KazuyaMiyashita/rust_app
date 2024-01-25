/// 割り込みを利用してボタンの入力をキューにためる
// use defmt::{info, warn, Format};

use alloc::boxed::Box;
use bsp::hal::{pac, pac::interrupt};
use core::cell::RefCell;
use critical_section::Mutex;
use rp_pico as bsp;
use rp_pico::hal::timer::{Alarm, Alarm1};
use rp_pico::hal::Timer;
use fugit::MicrosDurationU32;

const BUTTON_SCHEDULER_QUEUE_LENGTH: usize = 20;
pub struct Scheduler {
    buffer: [Box<dyn Fn() -> ()>; BUTTON_SCHEDULER_QUEUE_LENGTH],
    cursor: usize, // 次にバッファに書き込む位置。
    timer: Timer,
    alarm: Alarm1,
}

static GLOBAL_BUTTON_INPUT_QUEUE: Mutex<RefCell<Option<Scheduler>>> =
    Mutex::new(RefCell::new(None));

impl Scheduler {
    fn new (timer: Timer, alarm: Alarm1) -> Self {
        Scheduler {
            buffer: [|| (); BUTTON_SCHEDULER_QUEUE_LENGTH],
            cursor: 0,
            timer,
            alarm
        }
    }

    pub fn init(
        timer: Timer,
        mut alarm: Alarm1,
    ) {
        alarm.enable_interrupt();

        // Give away our pins by moving them into the `GLOBAL_PINS` variable.
        // We won't need to access them in the main thread again
        critical_section::with(|cs| {
          GLOBAL_BUTTON_INPUT_QUEUE
                .borrow(cs)
                .replace(Some(Scheduler::new(timer, alarm)));
        });

        // Unmask the IO_BANK0 IRQ so that the NVIC interrupt controller
        // will jump to the interrupt function when the interrupt occurs.
        // We do this last so that the interrupt can't go off while
        // it is in the middle of being configured
        unsafe {
            pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_1);
        }
    }

    pub fn set(countdown: MicrosDurationU32, f: Box<dyn Fn() -> ()>) {

      critical_section::with(|cs| {
        let mut button_input_queue_binding = GLOBAL_BUTTON_INPUT_QUEUE.borrow(cs).borrow_mut();
        let button_input_queue = button_input_queue_binding.as_mut().unwrap();

        button_input_queue.buffer[0] = f; // TODO
        button_input_queue.alarm.schedule(countdown).unwrap();
      })
    }

}

/// タイマーの割り込みが来た時、どのボタンによってタイマーが登録されたかを判断し、タイマーの割り込みのクリアおよび次のタイマーの設定を行う。
/// その後、そのボタンが正しく押され続けてたかを判断し、押されていたら押されたボタンキューに追加する
#[interrupt]
fn TIMER_IRQ_1() {
    critical_section::with(|cs| {
        let mut button_input_queue_binding = GLOBAL_BUTTON_INPUT_QUEUE.borrow(cs).borrow_mut();
        let button_input_queue = button_input_queue_binding.as_mut().unwrap();

        todo!();
    })
}
