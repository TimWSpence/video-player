use anyhow::Result;

mod decode;

fn main() -> Result<()> {
    decode::decode(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4",
    )?;
    Ok(())
}
