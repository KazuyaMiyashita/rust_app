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
        println!("{}", app_to_string(&app));

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