use anyhow::{anyhow, Result};
use ffmpeg::*;
use ffmpeg::{format, media};
use ffmpeg_next as ffmpeg;
use frame::Video;
use software::scaling::Flags;
use util::format::Pixel;

pub fn decode(file: &str) -> Result<Vec<Video>> {
    ffmpeg::init()?;

    let mut res = vec![];
    let mut count = 0;

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

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        video_decoder.format(),
        video_decoder.width(),
        video_decoder.height(),
        Pixel::RGBA,
        1920,
        1080,
        Flags::BILINEAR,
    )?;

    let mut process = |decoder: &mut ffmpeg::decoder::Video| -> Result<()> {
        let mut frame = Video::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            let mut rgb_frame = Video::empty();
            scaler.run(&frame, &mut rgb_frame)?;
            res.push(rgb_frame);
        }

        Ok(())
    };

    for (s, p) in input.packets() {
        count += 1;
        if count > 1000 {
            break;
        }
        if s.index() == video_idx {
            video_decoder.send_packet(&p)?;
            process(&mut video_decoder)?;
        }
    }
    video_decoder.send_eof()?;
    process(&mut video_decoder)?;

    Ok(res)
}
