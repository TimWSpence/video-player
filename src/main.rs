extern crate sdl2;

use anyhow::Result;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::Duration;

mod decode;

pub fn main() -> Result<()> {
    let (video_frames, audio_frames) = decode::decode(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
    )?;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 1920, 1080)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build()?;
    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 1920, 1080)?;

    canvas.set_draw_color(Color::RGB(255, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    for f in video_frames {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break,
                _ => {}
            }
        }

        canvas.with_texture_canvas(&mut texture, |t| {
            // t.clear();
            // t.set_draw_color(Color::RGB(255, 0, 0));
            // t.fill_rect(Rect::new(0, 0, 1920, 1080)).unwrap();
        })?;
        canvas.clear();
        texture.update(Rect::new(0, 0, 1920, 1080), f.data(0), 5760)?;
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        ::std::thread::sleep(Duration::from_millis(1000 / 60));
        // The rest of the game loop goes here...
    }

    Ok(())
}
