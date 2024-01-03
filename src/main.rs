extern crate termios;

use std::{io, thread};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::io::{Read, Write, Stdin};
use std::str;
use std::process;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use app::{App, Key};

fn main() {
    let app = Arc::new(Mutex::new(App::init()));

    let stdin = 0; // couldn't get std::os::unix::io::FromRawFd to work
    // on /dev/stdin or /dev/tty
    let termios = Termios::from_fd(stdin).unwrap();
    let mut new_termios = termios.clone();  // make a mutable copy of termios
    // that we will modify
    new_termios.c_lflag &= !(ICANON | ECHO); // no echo and canonical mode
    tcsetattr(stdin, TCSANOW, &mut new_termios).unwrap();

    let stdout = io::stdout();
    let mut reader = io::stdin();

    let start_time = Instant::now();

    println!("press arrow keys or press 'q' to exit");
    stdout.lock().flush().unwrap();

    let key_app = Arc::clone(&app);
    let key_thread = thread::spawn(move || {
        loop {
            match get_key_input_blocking(&mut reader) {
                Ok(key) => {
                    let app_elapsed_time = start_time.elapsed().as_millis();
                    key_app.lock().unwrap().on_key_down(key, app_elapsed_time)
                },
                Err(None) => process::exit(0),
                Err(Some(())) => continue
            }
        }
    });

    let display_app = Arc::clone(&app);
    let display_thread = thread::spawn(move || {
        let mut last_frame_time = Instant::now();

        loop {
            // 30fpsになるように制御
            let elapsed = last_frame_time.elapsed();
            if elapsed < Duration::from_millis(33) {
                thread::sleep(Duration::from_millis(33) - elapsed);
            }
            last_frame_time = Instant::now();

            let app_elapsed_time = start_time.elapsed().as_millis();
            // Clear the terminal using escape sequence
            print!("\x1B[2J\x1B[H"); // ESC[2JESC[H

            let total_durations = &display_app.lock().unwrap().format_total_durations_long(app_elapsed_time);

            println!("{}\n{}\n{}\n{}",
                     str::from_utf8(&total_durations[0]).unwrap(),
                     str::from_utf8(&total_durations[1]).unwrap(),
                     str::from_utf8(&total_durations[2]).unwrap(),
                     str::from_utf8(&total_durations[3]).unwrap(),
            );
        }
    });

    key_thread.join().unwrap();
    display_thread.join().unwrap();

    println!("finish");
    tcsetattr(stdin, TCSANOW, &termios).unwrap();  // reset the stdin to
    // original termios data
}

fn get_key_input_blocking(reader: &mut Stdin) -> Result<Key, Option<()>> {
    let mut buf = [0u8; 3];
    reader.read_exact(&mut buf[..1]).unwrap();

    if buf[0] == b'q' { return Err(None); }

    // Check if it's an escape sequence
    if buf[0] == 27 {
        // Read two more bytes to complete the escape sequence
        reader.read_exact(&mut buf[1..]).unwrap();

        // Process the escape sequence for arrow keys
        match &buf[..] {
            [27, 91, 65] => Ok(Key::AllowUp),
            [27, 91, 66] => Ok(Key::AllowDown),
            [27, 91, 67] => Ok(Key::AllowRight),
            [27, 91, 68] => Ok(Key::AllowLeft),
            _ => Err(Some(())),
        }
    } else { Err(Some(())) }
}
