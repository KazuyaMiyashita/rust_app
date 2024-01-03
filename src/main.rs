extern crate termios;

use std::io;
use std::io::Read;
use std::io::Write;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use app::{App, Key};

fn main() {
    let mut app = App::init();

    let stdin = 0; // couldn't get std::os::unix::io::FromRawFd to work
    // on /dev/stdin or /dev/tty
    let termios = Termios::from_fd(stdin).unwrap();
    let mut new_termios = termios.clone();  // make a mutable copy of termios
    // that we will modify
    new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin, TCSANOW, &mut new_termios).unwrap();

    let stdout = io::stdout();
    let mut reader = io::stdin();

    println!("press arrow keys or press 'q' to exit");
    stdout.lock().flush().unwrap();

    loop {
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf[..1]).unwrap();

        if buf[0] == b'q' { break; }

        // Check if it's an escape sequence
        if buf[0] == 27 {
            // Read two more bytes to complete the escape sequence
            reader.read_exact(&mut buf[1..]).unwrap();

            // Process the escape sequence for arrow keys
            match &buf[..] {
                [27, 91, 65] => app.on_key_down(Key::AllowUp),
                [27, 91, 66] => app.on_key_down(Key::AllowDown),
                [27, 91, 67] => app.on_key_down(Key::AllowRight),
                [27, 91, 68] => app.on_key_down(Key::AllowLeft),
                _ => (),
            }
        }

        // Clear the terminal using escape sequence
        print!("\x1B[2J\x1B[H"); // ESC[2JESC[H
        println!("{}", app_to_string(&app));
    }

    println!("finish");
    tcsetattr(stdin, TCSANOW, &termios).unwrap();  // reset the stdin to
    // original termios data
}

fn app_to_string(app: &App) -> String {
    let pad = " ".repeat(3 + app.cursor * 5);
    format!("[{:03}, {:03}, {:03}, {:03}]\n{}^",
            app.nums[0],
            app.nums[1],
            app.nums[2],
            app.nums[3],
            pad
    )
}