#![no_std] // Rust の標準ライブラリにリンクしない
#![no_main] // 全ての Rust レベルのエントリポイントを無効にする

use core::panic::PanicInfo;
use core::str;
use core::fmt::Write;
use pc_keyboard::PCKeyboard;
use vga_buffer::Writer;
use app::{App, Key};

mod vga_buffer;
mod pc_keyboard;

#[no_mangle] // この関数の名前修飾をしない
pub extern "C" fn _start() -> ! {
    // リンカはデフォルトで `_start` という名前の関数を探すので、
    // この関数がエントリポイントとなる

    let mut app = App::init();

    let mut keyboard = PCKeyboard::init();
    let mut writer = Writer::init();

    writeln!(writer, "Hello Waku-waku mi12cp World {}", 42).unwrap();
    writeln!(writer, "").unwrap();
    writeln!(writer, "ABCDEFGHIJKLMNOPQRSTUVWXYZ").unwrap();
    writeln!(writer, "abcdefghijklmnopqrstuvwxyz").unwrap();
    writeln!(writer, "1234567890").unwrap();


    loop {
        let c = keyboard.get_char_blocking();

        let app_elapsed_time: u128 = 0;
        match c {
            'a' => app.on_key_down(Key::AllowLeft, app_elapsed_time),
            's' => app.on_key_down(Key::AllowDown, app_elapsed_time),
            'd' => app.on_key_down(Key::AllowRight, app_elapsed_time),
            'w' => app.on_key_down(Key::AllowUp, app_elapsed_time),
            _ => ()
        }

        let total_durations = app.format_total_durations_long(app_elapsed_time);

        writer.clear_all();
        writeln!(writer, "{}\n{}\n{}\n{}",
                 str::from_utf8(&total_durations[0]).unwrap(),
                 str::from_utf8(&total_durations[1]).unwrap(),
                 str::from_utf8(&total_durations[2]).unwrap(),
                 str::from_utf8(&total_durations[3]).unwrap(),
        ).unwrap();
    }
}

/// この関数はパニック時に呼ばれる。
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}