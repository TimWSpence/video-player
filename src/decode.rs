use anyhow::{anyhow, Result};
use cpal::StreamConfig;
use ffmpeg::util::channel_layout::ChannelLayout;
use ffmpeg::*;
use ffmpeg::{format, media};
use ffmpeg_next as ffmpeg;
use frame::{Audio, Video};
use software::scaling::Flags;
use util::format::Pixel;

pub fn decode(file: &str, audio_cfg: &StreamConfig) -> Result<(Vec<Video>, Vec<Audio>)> {
    ffmpeg::init()?;

    let mut video_frames = vec![];
    let mut audio_frames = vec![];
    let mut count = 0;

    let mut input = format::input(file)?;

    let video = input
        .streams()
        .best(media::Type::Video)
        .ok_or(anyhow!("Could not find video stream"))?;
    let video_idx = video.index();

    let audio = input
        .streams()
        .best(media::Type::Audio)
        .ok_or(anyhow!("Could not find audio stream"))?;
    let audio_idx = audio.index();

    let video_ctx = ffmpeg::codec::context::Context::from_parameters(video.parameters())?;
    let mut video_decoder = video_ctx.decoder().video()?;

    let audio_ctx = ffmpeg::codec::context::Context::from_parameters(audio.parameters())?;
    let mut audio_decoder = audio_ctx.decoder().audio()?;
    let mut resampler = audio_decoder.resampler(
        format::Sample::F32(format::sample::Type::Packed),
        match audio_cfg.channels {
            1 => ChannelLayout::MONO,
            2 => ChannelLayout::STEREO,
            _ => panic!("Unsupported channel layout"),
        },
        audio_cfg.sample_rate.0,
    )?;

    println!(
        "Audio: sample rate {}, channels {}",
        audio_decoder.rate(),
        audio_decoder.channels()
    );

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        video_decoder.format(),
        video_decoder.width(),
        video_decoder.height(),
        Pixel::RGB24,
        1920,
        1080,
        Flags::BILINEAR,
    )?;

    let mut process_video = |decoder: &mut ffmpeg::decoder::Video| -> Result<()> {
        let mut frame = Video::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut rgb_frame = Video::empty();
            scaler.run(&frame, &mut rgb_frame)?;
            video_frames.push(rgb_frame);
        }

        Ok(())
    };

    let mut process_audio = |decoder: &mut ffmpeg::decoder::Audio| -> Result<()> {
        let mut frame = Audio::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut f = Audio::empty();
            let mut delay = resampler.run(&frame, &mut f)?;
            while let Some(_d) = delay {
                audio_frames.push(f.clone());
                delay = resampler.flush(&mut f)?;
            }
        }

        Ok(())
    };

    for (s, p) in input.packets() {
        count += 1;
        if count > 5000 {
            break;
        }
        if s.index() == video_idx {
            video_decoder.send_packet(&p)?;
            process_video(&mut video_decoder)?;
        } else if s.index() == audio_idx {
            audio_decoder.send_packet(&p)?;
            process_audio(&mut audio_decoder)?;
        }
    }
    video_decoder.send_eof()?;
    process_video(&mut video_decoder)?;
    audio_decoder.send_eof()?;
    process_audio(&mut audio_decoder)?;

    Ok((video_frames, audio_frames))
}
