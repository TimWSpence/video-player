use std::sync::mpsc::SyncSender;

use anyhow::{anyhow, Result};
use ffmpeg::*;
use ffmpeg::{format, media};
use ffmpeg_next as ffmpeg;
use frame::Video;
use software::scaling::Flags;
use util::format::Pixel;

pub struct Metadata {
    pub frame_rate: Rational,
    pub time_base: Rational,
}

pub fn metadata(file: &str) -> Result<Metadata> {
    let input = format::input(file)?;

    let video = input
        .streams()
        .best(media::Type::Video)
        .ok_or(anyhow!("Could not find video stream"))?;

    let video_ctx = ffmpeg::codec::context::Context::from_parameters(video.parameters())?;

    Ok(Metadata {
        frame_rate: video_ctx.frame_rate(),
        time_base: video.time_base(),
    })
}

pub fn decode(file: &str, buf: SyncSender<Video>) -> Result<()> {
    let mut input = format::input(file)?;

    let video = input
        .streams()
        .best(media::Type::Video)
        .ok_or(anyhow!("Could not find video stream"))?;
    let video_idx = video.index();

    let video_ctx = ffmpeg::codec::context::Context::from_parameters(video.parameters())?;
    let mut video_decoder = video_ctx.decoder().video()?;

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
            buf.send(rgb_frame.clone())?;
        }

        Ok(())
    };

    for (s, p) in input.packets() {
        if s.index() == video_idx {
            video_decoder.send_packet(&p)?;
            process_video(&mut video_decoder)?;
        }
    }
    video_decoder.send_eof()?;
    process_video(&mut video_decoder)?;

    Ok(())
}
