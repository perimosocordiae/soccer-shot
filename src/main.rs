use clap::Parser;
use mouse_rs::{
    types::{keys::Keys, Point},
    Mouse,
};
use screenshots::Screen;
use std::{fs, thread, time::Duration};
use xcb::x::{Drawable, GetImage, ImageFormat};

const GAME_WIDTH: u32 = 400;
const GAME_HEIGHT: u32 = 720;
const TARGET_RADIUS: i32 = 80;
const TARGET_DIAMETER: u32 = (TARGET_RADIUS * 2) as u32;

#[derive(Parser)]
struct Args {
    #[clap(long, default_value_t = 550)]
    game_x: i32,
    #[clap(long, default_value_t = 609)]
    game_y: i32,
    #[arg(value_enum, default_value = "none")]
    shot_type: ShotType,
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum ShotType {
    None,
    Center,
    Lob,
    Manual,
}

fn main() {
    let args = Args::parse();

    let screen = Screen::from_point(args.game_x, args.game_y).unwrap();
    println!("{:?}", screen);
    if screen.display_info.scale_factor != 1.0 {
        panic!("Pixel scaling not yet supported.");
    }
    if screen.display_info.x != 0 || screen.display_info.y != 0 {
        panic!("Multi-monitor not yet supported.");
    }

    let image = screen
        .capture_area(args.game_x, args.game_y, GAME_WIDTH, GAME_HEIGHT)
        .unwrap();
    fs::write("target/game.png", image.buffer()).unwrap();

    let mouse = Mouse::new();
    let orig_pos = mouse.get_position().unwrap();
    println!("mouse pos = {:?}", orig_pos);
    // Restore window focus.
    mouse.move_to(orig_pos.x, args.game_y - 5).unwrap();
    mouse.click(&Keys::LEFT).expect("Unable to click LMB");
    thread::sleep(Duration::from_millis(100));

    match args.shot_type {
        ShotType::None => {
            return;
        }
        ShotType::Manual => {
            // Return to original position.
            mouse.move_to(orig_pos.x, orig_pos.y).unwrap();
        }
        ShotType::Center => {
            let x = args.game_x + (GAME_WIDTH / 2) as i32;
            let y = args.game_y + (11 * GAME_HEIGHT / 16) as i32;
            mouse.move_to(x, y).unwrap();
        }
        ShotType::Lob => {
            let x = args.game_x + (GAME_WIDTH / 2) as i32;
            let y = args.game_y + GAME_HEIGHT as i32 - TARGET_RADIUS - 1;
            mouse.move_to(x, y).unwrap();
        }
    }

    // Take the shot.
    mouse.press(&Keys::LEFT).expect("Unable to press LMB");
    track_target(mouse.get_position().unwrap());
    mouse.release(&Keys::LEFT).expect("Unable to release LMB");

    // Restore the original mouse position.
    mouse.move_to(orig_pos.x, orig_pos.y).unwrap();
}

fn track_target(pos: Point) {
    let mut prev_red = 0;
    for i in 0..5000 {
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
        if num_red == 0 {
            continue;
        } else if num_red != prev_red {
            println!("target {}: num red = {}", i, num_red);
            prev_red = num_red;
        } else if num_red >= 6200 {
            break;
        }
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
