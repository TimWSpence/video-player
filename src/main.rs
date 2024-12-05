extern crate sdl2;

use anyhow::Result;

use cpal::traits::*;
use ffmpeg_next::frame::Audio;
use itertools::Itertools;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

mod decode;

pub fn main() -> Result<()> {
    let shutdown = AtomicBool::new(false);
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let audio_cfg = device.default_output_config()?.into();

    let (video_frames, audio_frames) = decode::decode(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
        &audio_cfg,
    )?;

    let audio_bytes: Vec<f32> = audio_frames
        .iter()
        .flat_map(|f| {
            f.data(0)
                .chunks_exact(4)
                .map(TryInto::try_into)
                .map(Result::unwrap)
                .map(f32::from_be_bytes)
        })
        .collect();
    let mut audio_bytes_idx = 0;

    let audio = device.build_output_stream(
        &audio_cfg,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for f in data {
                *f = audio_bytes[audio_bytes_idx];
                audio_bytes_idx += 1;
            }
        },
        |err| eprintln!("{}", err),
        None,
    )?;

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

    audio.play()?;

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
