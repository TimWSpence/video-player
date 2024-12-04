use anyhow::{anyhow, Result};
use decoder::video;
use ffmpeg::*;
use ffmpeg::{format, media};
use ffmpeg_next as ffmpeg;
use software::scaling::Flags;
use util::format::Pixel;

pub fn decode(file: &str) -> Result<()> {
    ffmpeg::init()?;

    let mut input = format::input(file)?;

    let video = input
        .streams()
        .best(media::Type::Video)
        .ok_or(anyhow!("Could not find video stream"))?;
    let video_idx = video.index();

    let _audio = input
        .streams()
        .best(media::Type::Audio)
        .ok_or(anyhow!("Could not find audio stream"))?;

    let video_ctx = ffmpeg::codec::context::Context::from_parameters(video.parameters())?;
    let mut video_decoder = video_ctx.decoder().video()?;

    let scaler = ffmpeg::software::scaling::context::Context::get(
        video_decoder.format(),
        video_decoder.width(),
        video_decoder.height(),
        Pixel::RGB24,
        1920,
        1080,
        Flags::BILINEAR,
    )?;

    let mut process = |decoder: &mut ffmpeg::decoder::Video| -> Result<()> { Ok(()) };

    for (s, p) in input.packets() {
        if s.index() == video_idx {
            video_decoder.send_packet(&p)?;
            process(&mut video_decoder)?;
        }
    }
    video_decoder.send_eof()?;
    process(&mut video_decoder)?;

    Ok(())
}
