use mouse_rs::{types::keys::Keys, Mouse};
use screenshots::Screen;
use std::{fs, time::Instant};

fn main() {
    let start = Instant::now();
    let screens = Screen::all().unwrap();

    for screen in screens {
        println!("capturer {:?}", screen);
        let mut image = screen.capture().unwrap();
        let mut buffer = image.buffer();
        fs::write(format!("target/{}.png", screen.display_info.id), buffer).unwrap();

        image = screen.capture_area(300, 300, 300, 300).unwrap();
        buffer = image.buffer();
        fs::write(format!("target/{}-2.png", screen.display_info.id), buffer).unwrap();
    }

    let screen = Screen::from_point(100, 100).unwrap();
    println!("capturer {:?}", screen);

    let image = screen.capture_area(300, 300, 300, 300).unwrap();
    let buffer = image.buffer();
    fs::write("target/capture_display_with_point.png", buffer).unwrap();

    println!("elapsed time: {:?}", start.elapsed());

    let mouse = Mouse::new();
    mouse.move_to(500, 500).expect("Unable to move mouse");
    mouse.press(&Keys::RIGHT).expect("Unable to press button");
    mouse
        .release(&Keys::RIGHT)
        .expect("Unable to release button");
}
