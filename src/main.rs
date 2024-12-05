extern crate sdl2;

use anyhow::Result;

use ffmpeg_next::frame::Audio;
use sdl2::audio::{AudioCallback, AudioQueue, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

mod decode;

struct AudioData {
    frames: Vec<Audio>,
}
// format is F32 Planar

impl AudioCallback for AudioData {
    type Channel = f32;

    fn callback(&mut self, _data: &mut [f32]) {
        todo!()
    }
}

pub fn main() -> Result<()> {
    let (video_frames, audio_frames) = decode::decode(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
    )?;

    let shutdown = AtomicBool::new(false);
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44_100),
        channels: Some(2),
        samples: None,
    };

    let audio_device: AudioQueue<u8> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
    for f in audio_frames {
        audio_device.queue_audio(f.data(0)).unwrap();
    }

    let window = video_subsystem
        .window("hacky video player", 1920, 1080)
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
    audio_device.resume();

    'main: for f in video_frames {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => {}
            }
        }
        canvas.with_texture_canvas(&mut texture, |_t| {})?;
        canvas.clear();
        texture.update(Rect::new(0, 0, 1920, 1080), f.data(0), 5760)?;
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        ::std::thread::sleep(Duration::from_millis(1000 / 60));
    }

    shutdown.store(true, Ordering::Release);

    Ok(())
}
