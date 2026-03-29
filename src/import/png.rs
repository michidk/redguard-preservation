use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageError};
use std::io::BufWriter;
use std::path::Path;

pub fn save_png(image: &DynamicImage, path: &Path, compress: bool) -> Result<(), ImageError> {
    let file = std::fs::File::create(path)?;
    let w = BufWriter::new(file);
    let (compression, filter) = if compress {
        (CompressionType::Default, FilterType::Adaptive)
    } else {
        (CompressionType::Fast, FilterType::NoFilter)
    };
    let encoder = PngEncoder::new_with_quality(w, compression, filter);
    image.write_with_encoder(encoder)
}
