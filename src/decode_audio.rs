use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use cpal::{SupportedStreamConfig, SupportedStreamConfigRange};
use ffmpeg::util::channel_layout::ChannelLayout;
use ffmpeg::*;
use ffmpeg::{format, media};
use ffmpeg_next as ffmpeg;
use frame::Audio;
use ringbuf::*;

pub fn decode(file: &str, audio_cfg: &SupportedStreamConfig, mut buf: Producer<f32>) -> Result<()> {
    let mut input = format::input(file)?;

    let audio = input
        .streams()
        .best(media::Type::Audio)
        .ok_or(anyhow!("Could not find audio stream"))?;
    let audio_idx = audio.index();

    let audio_ctx = ffmpeg::codec::context::Context::from_parameters(audio.parameters())?;
    let mut audio_decoder = audio_ctx.decoder().audio()?;
    let mut resampler = audio_decoder.resampler(
        format::Sample::F32(format::sample::Type::Packed),
        ffmpeg::ChannelLayout::default(audio_cfg.channels().into()),
        audio_cfg.sample_rate().0,
    )?;

    let mut process_audio = |decoder: &mut ffmpeg::decoder::Audio| -> Result<()> {
        let mut frame = Audio::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut f = Audio::empty();
            let mut delay = resampler.run(&frame, &mut f)?;
            loop {
                blocking_write(ffmpeg_frame_to_slice(&f), &mut buf)?;
                if delay.is_none() {
                    break;
                }
                delay = resampler.flush(&mut f)?;
            }
        }

        Ok(())
    };

    for (s, p) in input.packets() {
        if s.index() == audio_idx {
            audio_decoder.send_packet(&p)?;
            process_audio(&mut audio_decoder)?;
        }
    }
    audio_decoder.send_eof()?;
    process_audio(&mut audio_decoder)?;

    Ok(())
}

fn ffmpeg_frame_to_slice<T: frame::audio::Sample>(frame: &frame::Audio) -> &[T] {
    if !frame.is_packed() {
        panic!("Frame is not packed");
    }

    if !T::is_valid(frame.format(), frame.channels()) {
        panic!("Invalid frame");
    }

    unsafe {
        std::slice::from_raw_parts(
            (*frame.as_ptr()).data[0] as *const T,
            frame.samples() * frame.channels() as usize,
        )
    }
}

fn blocking_write<T: Copy>(data: &[T], buf: &mut Producer<T>) -> Result<()> {
    while buf.remaining() < data.len() {
        thread::sleep(Duration::from_millis(10));
    }
    buf.push_slice(data);
    Ok(())
}
