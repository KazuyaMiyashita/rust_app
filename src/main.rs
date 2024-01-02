mod app;

use std::{io, process};
use app::{App, Key};

fn main() {

    println!("Hello, world!
カウンターが4つもついたすごいアプリだよ

矢印キーのように a, w, s, d のどれかを入力してね
q を入力すると終了するよ
");

    let mut app = App::init();

    loop {
        println!("{}", app.to_string());

        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();

        match line.trim() {
            "a" => app.on_key_down(Key::AllowLeft),
            "s" => app.on_key_down(Key::AllowDown),
            "d" => app.on_key_down(Key::AllowRight),
            "w" => app.on_key_down(Key::AllowUp),
            "q" => process::exit(0),
            other => println!("unrecognized key: {}", other)
        }
    }

}
