use anyhow::Result;
use sdl2::rect::Rect;
use sdl2::render::TextureAccess;
use std::thread;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use std::time::Duration;

mod decode;

fn main() -> Result<()> {
    let frames = decode::decode(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
    )?;

    println!("Frames: {}", frames.len());

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("hacky video player", 1920, 1080)
        .position_centered()
        .build()?;
    let mut canvas = window.into_canvas().build()?;
    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();

    for f in frames {
        thread::sleep(Duration::from_millis(10));
        let c = canvas.texture_creator();
        let mut texture =
            c.create_texture(PixelFormatEnum::RGB24, TextureAccess::Target, 1920, 1080)?;
        // texture.update(Rect::new(0, 0, 1920, 1080), f.data(0), f.data(0).len())?;
        canvas.with_texture_canvas(&mut texture, |t| {
            t.set_draw_color(Color::RGBA(0, 0, 0, 255));
            t.clear();
            t.set_draw_color(Color::RGBA(255, 0, 0, 255));
            t.fill_rect(Rect::new(50, 50, 50, 50)).unwrap();
        })?;
        canvas.present();
    }

    Ok(())
}
