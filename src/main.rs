use mouse_rs::{types::keys::Keys, Mouse};
use screenshots::Screen;
use std::{fs, thread, time::Duration};
use xcb::x::{Drawable, GetImage, ImageFormat};

const GAME_WIDTH: u32 = 400;
const GAME_HEIGHT: u32 = 720;
const GAME_X: i32 = 550;
const GAME_Y: i32 = 609;
const TARGET_RADIUS: i32 = 80;
const TARGET_DIAMETER: u32 = (TARGET_RADIUS * 2) as u32;

fn main() {
    let screen = Screen::from_point(GAME_X, GAME_Y).unwrap();
    println!("Screen: {:?}", screen);
    if screen.display_info.scale_factor != 1.0 {
        panic!("Pixel scaling not yet supported.");
    }
    if screen.display_info.x != 0 || screen.display_info.y != 0 {
        panic!("Multi-monitor not yet supported.");
    }

    let image = screen
        .capture_area(GAME_X, GAME_Y, GAME_WIDTH, GAME_HEIGHT)
        .unwrap();
    fs::write("target/game.png", image.buffer()).unwrap();

    let mouse = Mouse::new();
    let pos = mouse.get_position().unwrap();
    println!("mouse pos = {:?}", pos);
    // Restore window focus.
    mouse.move_to(pos.x, GAME_Y - 5).unwrap();
    mouse.click(&Keys::LEFT).expect("Unable to click LMB");
    thread::sleep(Duration::from_millis(100));

    // Return to original position and start the shot.
    mouse.move_to(pos.x, pos.y).unwrap();
    mouse.press(&Keys::LEFT).expect("Unable to press LMB");

    // Track the target.
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
            .filter(|bgra| bgra[2] > 200 && bgra[1] < 100 && bgra[0] < 100)
            .count();
        if num_red != prev_red {
            println!("target {}: num red = {}", i, num_red);
            prev_red = num_red;
        } else if num_red >= 5500 {
            break;
        }
    }
    mouse.release(&Keys::LEFT).expect("Unable to release LMB");
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
