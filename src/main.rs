use clap::Parser;
use mouse_rs::{
    types::{keys::Keys, Point},
    Mouse,
};
use screenshots::Screen;
use std::{
    fs,
    io::{stdin, stdout, Write},
    thread,
    time::{Duration, Instant},
};
use xcb::x::{Drawable, GetImage, ImageFormat};

const GAME_WIDTH: u32 = 400;
const GAME_HEIGHT: u32 = 720;
const TARGET_RADIUS: i32 = 80;
const TARGET_DIAMETER: u32 = (TARGET_RADIUS * 2) as u32;
const TIME_PER_FRAME: Duration = Duration::from_millis(16); // 60 FPS
const BALL_PATTERN: [u8; 48] = [
    31, 11, 17, 0, // black
    102, 66, 55, 0, // gray
    255, 255, 255, 0, 255, 255, 255, 0, // white (x2)
    102, 66, 55, 0, 102, 66, 55, 0, 102, 66, 55, 0, 102, 66, 55, 0, // gray (x4)
    255, 255, 255, 0, 255, 255, 255, 0, // white (x2)
    102, 66, 55, 0, // gray
    31, 11, 17, 0, // black
];

#[derive(Parser)]
struct Args {
    #[clap(long, default_value_t = 550)]
    game_x: i32,
    #[clap(long, default_value_t = 609)]
    game_y: i32,
    #[clap(long, default_value_t = false)]
    ready: bool,
}

enum ShotType {
    Center,
    Lob,
    Manual,
}

fn main() {
    let args = Args::parse();

    let screen = Screen::from_point(args.game_x, args.game_y).unwrap();
    if screen.display_info.scale_factor != 1.0 {
        panic!("Pixel scaling not yet supported: {:?}", screen.display_info);
    }
    if screen.display_info.x != 0 || screen.display_info.y != 0 {
        panic!("Multi-monitor not yet supported: {:?}", screen.display_info);
    }

    let mouse = Mouse::new();
    if !args.ready {
        println!("{:?}", screen.display_info);
        println!("mouse pos = {:?}", mouse.get_position().unwrap());
        let image = screen
            .capture_area(args.game_x, args.game_y, GAME_WIDTH, GAME_HEIGHT)
            .unwrap();
        fs::write("target/game.png", image.to_png().unwrap()).unwrap();
        return;
    }

    let center_x = args.game_x + (GAME_WIDTH / 2) as i32;
    let help_text = "Shot types: [a]uto, [c]enter, [l]ob, [m]anual, [q]uit";
    println!("{}", help_text);
    let mut input_buffer = String::new();
    loop {
        print!("> ");
        stdout().flush().unwrap();
        stdin().read_line(&mut input_buffer).unwrap();
        match input_buffer.trim() {
            "a" => {
                aim_shot(&mouse, center_x, args.game_x, args.game_y);
                thread::sleep(Duration::from_millis(500));
                take_shot(&mouse, center_x, args.game_y, ShotType::Center);
            }
            "c" => take_shot(&mouse, center_x, args.game_y, ShotType::Center),
            "l" => take_shot(&mouse, center_x, args.game_y, ShotType::Lob),
            "m" => take_shot(&mouse, center_x, args.game_y, ShotType::Manual),
            "q" => break,
            _ => println!("Invalid input: '{:?}'. {}", &input_buffer, help_text),
        }
        input_buffer.clear();
    }
}

fn aim_shot(mouse: &Mouse, center_x: i32, game_x: i32, game_y: i32) {
    let orig_pos = restore_focus(mouse, center_x, game_y);

    // Locate the soccer ball by sliding the BALL_PATTERN across the screen.
    let bgra_pixels = capture(game_x, game_y, GAME_WIDTH, GAME_HEIGHT).unwrap();
    let (raw_idx, _) = bgra_pixels
        .windows(BALL_PATTERN.len())
        .enumerate()
        .min_by_key(|&(_, window)| {
            let mut diff = 0;
            for (j, pixel) in window.iter().enumerate() {
                diff += pixel.abs_diff(BALL_PATTERN[j]) as u32;
            }
            diff
        })
        .unwrap();
    let idx = (raw_idx + BALL_PATTERN.len() / 2) / 4;
    let ball_x = game_x + (idx % GAME_WIDTH as usize) as i32;
    let ball_y = game_y + (idx / GAME_WIDTH as usize) as i32;
    println!("ball = ({}, {})", ball_x, ball_y);

    // Compute the vector from orig_pos to the ball.
    let dx = ball_x - orig_pos.x;
    let dy = ball_y - orig_pos.y;
    // Scale to length 125.
    let scale = 125.0 / ((dx * dx + dy * dy) as f32).sqrt();
    let dx = (dx as f32 * scale) as i32;
    let dy = (dy as f32 * scale) as i32;

    // Aim the shot.
    mouse.press(&Keys::LEFT).unwrap();
    thread::sleep(Duration::from_millis(100));
    mouse.move_to(orig_pos.x + dx, orig_pos.y + dy).unwrap();
    thread::sleep(Duration::from_millis(100));
    mouse.release(&Keys::LEFT).unwrap();
}

fn take_shot(mouse: &Mouse, center_x: i32, game_y: i32, shot_type: ShotType) {
    let orig_pos = restore_focus(mouse, center_x, game_y);
    match shot_type {
        ShotType::Manual => {
            // Return to original position.
            mouse.move_to(orig_pos.x, orig_pos.y).unwrap();
        }
        ShotType::Center => {
            let y = game_y + (11 * GAME_HEIGHT / 16) as i32;
            mouse.move_to(center_x, y).unwrap();
        }
        ShotType::Lob => {
            let y = game_y + GAME_HEIGHT as i32 - TARGET_RADIUS - 1;
            mouse.move_to(center_x, y).unwrap();
        }
    }

    // Take the shot.
    mouse.press(&Keys::LEFT).expect("Unable to press LMB");
    track_target(mouse.get_position().unwrap());
    mouse.release(&Keys::LEFT).expect("Unable to release LMB");

    // Restore the original mouse position.
    mouse.move_to(orig_pos.x, orig_pos.y).unwrap();
}

fn restore_focus(mouse: &Mouse, center_x: i32, game_y: i32) -> Point {
    let orig_pos = mouse.get_position().unwrap();
    mouse.move_to(center_x, game_y - 5).unwrap();
    mouse.click(&Keys::LEFT).expect("Unable to click LMB");
    thread::sleep(Duration::from_millis(50));
    mouse.move_to(orig_pos.x, orig_pos.y).unwrap();
    orig_pos
}

fn track_target(pos: Point) {
    std::thread::sleep(TIME_PER_FRAME * 10);
    let mut frame_time = Instant::now() + TIME_PER_FRAME;
    let mut baseline = 0;
    for i in 0..60 {
        let bgra_pixels = capture(
            pos.x - TARGET_RADIUS,
            pos.y - TARGET_RADIUS,
            TARGET_DIAMETER,
            TARGET_DIAMETER,
        )
        .unwrap();
        // Count red pixels
        let num_red = bgra_pixels
            .chunks_exact(4)
            .filter(|bgra| bgra[2].saturating_sub(bgra[1]).saturating_sub(bgra[0]) > 0)
            .count();
        if baseline == 0 {
            baseline = num_red;
            println!("baseline = {}", baseline);
        } else if num_red > baseline + 300 {
            println!("target {}: num red = {}", i, num_red);
            break;
        }
        std::thread::sleep(frame_time.saturating_duration_since(Instant::now()));
        frame_time += TIME_PER_FRAME;
    }
}

fn capture(x: i32, y: i32, width: u32, height: u32) -> Option<Vec<u8>> {
    let (conn, index) = xcb::Connection::connect(None).ok()?;
    let screen = conn.get_setup().roots().nth(index as usize)?;

    let get_image_cookie = conn.send_request(&GetImage {
        format: ImageFormat::ZPixmap,
        drawable: Drawable::Window(screen.root()),
        x: x as i16,
        y: y as i16,
        width: width as u16,
        height: height as u16,
        plane_mask: u32::MAX,
    });

    let get_image_reply = conn.wait_for_reply(get_image_cookie).ok()?;
    // Returns pixel data in BGRA format.
    Some(Vec::from(get_image_reply.data()))
}
