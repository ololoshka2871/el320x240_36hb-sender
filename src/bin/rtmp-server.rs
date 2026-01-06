use ez_ffmpeg::{AVRational, FfmpegContext, Input, Output};

use const_format::concatcp;

use serialport::SerialPort;
use structopt::StructOpt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use el320x240_36hb_sender::{DitherAlgorithm, TempPixelFormat};

pub const DISPLAY_WIDTH: u32 = 320;
pub const DISPLAY_HEIGHT: u32 = 240;
pub const DISPLAY_FPS: u32 = 30;

#[allow(unused)]
#[derive(Debug, StructOpt)]
#[structopt(about = "Send rtmp stream to el320x240_36hb over virtual serial port")]
pub struct Cli {
    /// width
    #[structopt(long, default_value = concatcp!(DISPLAY_WIDTH))]
    pub width: u32,

    /// heigth
    #[structopt(long, default_value = concatcp!(DISPLAY_HEIGHT))]
    pub heigth: u32,

    /// fps
    #[structopt(long, default_value = concatcp!(DISPLAY_FPS))]
    pub fps: u32,

    /// RTMP port
    #[structopt(short, default_value = "9009")]
    pub rtmp_port: u16,

    /// Serial port
    #[structopt(short, default_value = "/dev/ttyACM0")]
    pub serial_port: String,

    /// use filter algorithm
    #[structopt(short, default_value = "bayer")]
    pub filter_algorithm: DitherAlgorithm,

    /// temp pixel format
    #[structopt(short, default_value = "monob")]
    pub temp_pixel_format: TempPixelFormat,
}

#[tokio::main]
async fn main() -> ez_ffmpeg::error::Result<()> {
    let args = Cli::from_args();

    // 1. Prepare an `Input` pointing to a local file (e.g., "test.mp4")
    let input: Input = Input::from(format!("rtmp://127.0.0.1:{}", args.rtmp_port))
        .set_readrate(1.0)
        .set_input_opt("listen", "1")
        .set_input_opt("flags", "nobuffer")
        .set_input_opt("tune", "zerolatency");

    // 2. todo: filters from config, like "diter" and so on
    let filter = format!(
        r#"
        [0]format={}[a];
        [a]split[m][t];
        [t]palettegen=max_colors=2:reserve_transparent=0:stats_mode=single[p];
        [m][p]paletteuse=dither={}:new=1[g];
        [g]format=gray
    "#,
        <TempPixelFormat as Into<&'static str>>::into(args.temp_pixel_format),
        <DitherAlgorithm as Into<&'static str>>::into(args.filter_algorithm)
    );
    let (mut reader, mut writer) = tokio::io::simplex(150 * 1024); // Specify a buffer capacity

    // 3. output: Define the write callback for custom output handling
    let write_callback = move |buf: &[u8]| -> i32 {
        if buf.is_empty() {
            return 0;
        }

        match futures::executor::block_on(writer.write_all(buf)) {
            Err(_e) => ffmpeg_sys_next::AVERROR(ffmpeg_sys_next::AVFMT_FLAG_NOBUFFER),
            Ok(_) => buf.len() as i32,
        }
    };

    // https://ffmpeg.org/ffmpeg-formats.html#rawvideo
    let output = Output::new_by_write_callback(write_callback)
        .set_format("rawvideo")
        .set_framerate(AVRational {
            num: args.fps as i32,
            den: 1,
        })
        .set_video_codec_opts(vec![
            ("framerate", &format!("{}", args.fps)),
            ("video_size", &format!("{}x{}", args.width, args.heigth)),
        ])
        .set_bits_per_raw_sample(8);

    let mut port = serialport::new(&args.serial_port, 15000000)
        .timeout(tokio::time::Duration::from_millis(5))
        .open()
        .expect("Failed to open serial port");

    tokio::spawn(async move {
        let mut buf = Vec::with_capacity(args.width as usize * args.heigth as usize);
        let mut bit_buf = Vec::with_capacity(buf.capacity() / 8);
        buf.resize(buf.capacity(), 0u8);
        bit_buf.resize(bit_buf.capacity(), 0u8);

        loop {
            match reader.read_exact(&mut buf).await {
                Ok(_) => {
                    buf.chunks_exact(8).enumerate().for_each(|(i, chunk)| {
                        let mut byte: u8 = 0;
                        for (j, &pixel) in chunk.iter().enumerate() {
                            let bit = if pixel > 128 { 1 } else { 0 };
                            byte |= bit << (7 - j);
                        }
                        bit_buf[i] = byte;
                    });
                    display_send(port.as_mut(), (args.width, args.heigth), &bit_buf).await;
                }
                Err(e) => {
                    eprintln!("Error reading from pipe: {}", e);
                    break;
                }
            }
        }
    });

    println!("Starting RTMP server on port 127.0.0.1:{}", args.rtmp_port);

    // 4. Build and run the FFmpeg context
    FfmpegContext::builder()
        .input(input)
        .output(output)
        .filter_desc(filter)
        .build()?
        .start()?
        .await
}

async fn display_send<P: SerialPort + ?Sized>(port: &mut P, (w, h): (u32, u32), frame: &[u8]) {
    use bytes::{BufMut, BytesMut};

    let line_size_bytes = (w / 8) as usize;
    assert!(
        frame.len() == line_size_bytes * h as usize,
        "Frame size does not match specified width and height"
    );

    frame
        .chunks_exact(line_size_bytes)
        .enumerate()
        .for_each(|(line, data)| {
            let offset_bytes = line * line_size_bytes;
            let mut buf: BytesMut = BytesMut::new();

            // offset in bytes
            buf.put_u32_le(offset_bytes as u32);
            buf.put_slice(data);

            port.write_all(&buf).unwrap();
        });
}
