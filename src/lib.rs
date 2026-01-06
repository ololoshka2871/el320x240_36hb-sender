#[derive(Debug, Clone, Copy)]
pub enum DitherAlgorithm {
    Bayer,
    Heckbert,
    FloydSteinberg,
    Sierra2,
    Sierra2_4a,
    Sierra3,
    Burkes,
    Atkinson,
}

impl std::str::FromStr for DitherAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bayer" => Ok(DitherAlgorithm::Bayer),
            "heckbert" => Ok(DitherAlgorithm::Heckbert),
            "floydSteinberg" => Ok(DitherAlgorithm::FloydSteinberg),
            "sierra2" => Ok(DitherAlgorithm::Sierra2),
            "sierra2_4a" => Ok(DitherAlgorithm::Sierra2_4a),
            "sierra3" => Ok(DitherAlgorithm::Sierra3),
            "burkes" => Ok(DitherAlgorithm::Burkes),
            "atkinson" => Ok(DitherAlgorithm::Atkinson),
            _ => Err(format!(
                "Unknown dithering algorithm: {}, mast be one of {}",
                s, "Threshold, Ordered or Pinwheel"
            )),
        }
    }
}

impl Into<&'static str> for DitherAlgorithm {
    fn into(self) -> &'static str {
        match self {
            DitherAlgorithm::Bayer => "bayer",
            DitherAlgorithm::Heckbert => "heckbert",
            DitherAlgorithm::FloydSteinberg => "floydSteinberg",
            DitherAlgorithm::Sierra2 => "sierra2",
            DitherAlgorithm::Sierra2_4a => "sierra2_4a",
            DitherAlgorithm::Sierra3 => "sierra3",
            DitherAlgorithm::Burkes => "burkes",
            DitherAlgorithm::Atkinson => "atkinson",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TempPixelFormat {
    MonoB,
    MonoW,
}

impl std::str::FromStr for TempPixelFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "monob" => Ok(TempPixelFormat::MonoB),
            "monow" => Ok(TempPixelFormat::MonoW),
            _ => Err(format!(
                "Unknown temp pixel format: {}, must be one of monob or monow",
                s
            )),
        }
    }
}

impl Into<&'static str> for TempPixelFormat {
    fn into(self) -> &'static str {
        match self {
            TempPixelFormat::MonoB => "monob",
            TempPixelFormat::MonoW => "monow",
        }
    }
}