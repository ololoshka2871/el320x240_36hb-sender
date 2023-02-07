use structopt::StructOpt;
use const_format::concatcp;

pub const DISPLAY_WIDTH: u32 = 320;
pub const DISPLAY_HEIGHT: u32 = 240;
pub const DISPLAY_FPS: u32 = 30;

#[derive(Debug)]
pub enum DitherAlgorithm {
    Threshold,
    Ordered,
    Pinwheel,
}

#[allow(unused)]
#[derive(Debug, StructOpt)]
#[structopt(
    about = "Send image from OBS virtual camera to el320x240_36hb over virtual serial port"
)]
pub struct Cli {
    /// camera ID
    #[structopt(short, default_value = "0")]
    pub id: u32,

    /// width
    #[structopt(long, default_value = concatcp!(DISPLAY_WIDTH))]
    pub width: u32,

    /// heigth
    #[structopt(long, default_value = concatcp!(DISPLAY_HEIGHT))]
    pub heigth: u32,

    /// fps
    #[structopt(long, default_value = concatcp!(DISPLAY_FPS))]
    pub fps: u32,

    /// Serial port
    #[structopt(short, default_value = "/dev/ttyACM0")]
    pub port: String,

    /// use filter algorithm
    #[structopt(short, default_value = "Threshold")]
    pub filter_algorithm: DitherAlgorithm,

    /// level of black color, 0.0-1.0
    #[structopt(long, default_value = "0.0", parse(try_from_str = parse_h_val))]
    pub black_lvl: f32,

    /// level of white color, 0.0-1.0
    #[structopt(long, default_value = "1.0", parse(try_from_str = parse_h_val))]
    pub white_lvl: f32,

    /// list available cameras
    #[structopt(short)]
    pub list: bool,
}

impl std::str::FromStr for DitherAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Threshold" => Ok(DitherAlgorithm::Threshold),
            "Ordered" => Ok(DitherAlgorithm::Ordered),
            "Pinwheel" => Ok(DitherAlgorithm::Pinwheel),
            _ => Err(format!("Unknown dithering algorithm: {}, mast be one of {}", s, "Threshold, Ordered or Pinwheel")),
        }
    }
}

fn parse_h_val(s: &str) -> Result<f32, String> {
    let val = s.parse::<f32>().map_err(|e| e.to_string())?;
    if val < 0.0 || val > 1.0 {
        Err(format!("Value must be in range 0.0-1.0, got {val}"))
    } else {
        Ok(val)
    }
}
