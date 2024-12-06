extern crate sdl2;

use anyhow::Result;

use cpal::{traits::*, Sample};
use ringbuf::RingBuffer;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::Duration;

mod decode_audio;
mod decode_video;

pub fn main() -> Result<()> {
    let file = "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4";
    // let file = "https://ia903405.us.archive.org/27/items/archive-video-files/test.mp4";
    // let file = "video.mp4";

    let shutdown = AtomicBool::new(false);
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let audio_cfg: cpal::SupportedStreamConfig = device
        .supported_output_configs()
        .unwrap()
        .next()
        .unwrap()
        .with_max_sample_rate();

    let (video_producer, video_consumer) = sync_channel(8192);

    let video_thread = thread::spawn(|| decode_video::decode(file, video_producer).unwrap());

    let audio_buf = RingBuffer::<f32>::new(16384);
    let (audio_producer, mut audio_consumer) = audio_buf.split();

    let cfg = audio_cfg.clone();
    let audio_thread = thread::spawn(move || {
        decode_audio::decode(file, &cfg, audio_producer).unwrap();
    });

    let audio = device.build_output_stream(
        &audio_cfg.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for f in data {
                *f = match audio_consumer.pop() {
                    Some(t) => t,
                    None => Sample::from_sample(0.0f32),
                }
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

    'main: loop {
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

        match video_consumer.recv() {
            Ok(f) => {
                canvas.with_texture_canvas(&mut texture, |_t| {})?;
                canvas.clear();
                texture.update(Rect::new(0, 0, 1920, 1080), f.data(0), 5760)?;
                canvas.copy(&texture, None, None).unwrap();
                canvas.present();
                ::std::thread::sleep(Duration::from_millis(1000 / 60));
            }
            _ => break 'main,
        }
    }

    shutdown.store(true, Ordering::Release);

    audio_thread.join().unwrap();
    video_thread.join().unwrap();

    Ok(())
}
