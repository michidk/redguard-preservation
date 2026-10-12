//! Indexed pixels shared by PCX, LBM, and ANM images.

use crate::{Result, error::Error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedImage {
    pub width: u16,
    pub height: u16,
    pub palette: [[u8; 3]; 256],
    pub pixels: Vec<u8>,
    pub transparent_index: Option<u8>,
}

impl IndexedImage {
    pub fn rgba(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|&index| {
                let [r, g, b] = self.palette[usize::from(index)];
                [
                    r,
                    g,
                    b,
                    if self.transparent_index == Some(index) {
                        0
                    } else {
                        255
                    },
                ]
            })
            .collect()
    }
}

/// Bound decoded images to 16 Mi pixels before allocating from untrusted headers.
pub(crate) fn pixel_count(width: u16, height: u16) -> Result<usize> {
    let count = usize::from(width) * usize::from(height);
    if count == 0 || count > 16 * 1024 * 1024 {
        return Err(Error::Parse(
            "Image dimensions must contain 1..16777216 pixels".into(),
        ));
    }
    Ok(count)
}
