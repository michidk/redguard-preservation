//! Indexed Glide allocation and sampling from docs/engine/model-lighting.md.
use crate::{Result, error::Error};

/// Source dimensions, reduced allocation and nearest-source sampling strides.
///
/// Allocation padding contains palette index zero. The source image and its
/// authored geometry retain their original dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlideTextureLayout {
    source: [u16; 2],
    allocation: [u16; 2],
    stride: [u16; 2],
}

impl GlideTextureLayout {
    /// Construct the BSI loading layout for dimensions in `1..=256`.
    ///
    /// `text_scale` is SYSTEM.INI's `[3dfx]` pair, ordered 256-size then
    /// 128-size. An absent setting uses the engine default `[4, 2]`.
    /// Only positive power-of-two factors no greater than their corresponding
    /// allocation size are supported; other settings return a contextual error.
    pub fn new(source: [u16; 2], text_scale: Option<[i32; 2]>) -> Result<Self> {
        let error = |reason: &str| {
            Error::Parse(format!(
                "Glide texture {}x{}: {reason}",
                source[0], source[1]
            ))
        };
        if source.iter().any(|&v| v == 0 || v > 256) {
            return Err(error("source dimensions must be in 1..=256"));
        }
        let factors = text_scale.unwrap_or([4, 2]);
        for (factor, limit) in factors.into_iter().zip([256, 128]) {
            if factor <= 0 || factor > limit || !(factor as u32).is_power_of_two() {
                return Err(error("unsupported SYSTEM.INI [3dfx] text_scale factor"));
            }
        }
        let rounded = source.map(u16::next_power_of_two);
        let stride = rounded.map(|size| match size {
            256 => factors[0] as u16,
            128 => factors[1] as u16,
            _ => 1,
        });
        let mut allocation = std::array::from_fn(|i| rounded[i] / stride[i]);
        let minimum = allocation[0].max(allocation[1]) / 8;
        allocation = allocation.map(|size| size.max(minimum));
        Ok(Self {
            source,
            allocation,
            stride,
        })
    }

    /// Original image dimensions, before reduction or padding.
    #[must_use]
    pub fn source_dimensions(self) -> [u16; 2] {
        self.source
    }

    /// Power-of-two allocation dimensions, including aspect-ratio padding.
    #[must_use]
    pub fn allocation_dimensions(self) -> [u16; 2] {
        self.allocation
    }

    /// Horizontal and vertical source-texel strides, starting at source zero.
    #[must_use]
    pub fn sampling_stride(self) -> [u16; 2] {
        self.stride
    }

    /// Scale from original pixel-space coordinates to Glide S/T units.
    /// Integer division of the source dimension happens before scaling.
    #[must_use]
    pub fn coordinate_scale(self) -> [f32; 2] {
        let maximum = f64::from(self.allocation[0].max(self.allocation[1]));
        std::array::from_fn(|i| {
            (f64::from(self.source[i] / self.stride[i]) * 256.0
                / (maximum * f64::from(self.source[i]))) as f32
        })
    }

    /// Fixed Glide sampling level: zero represents a 256-texel maximum axis.
    #[must_use]
    pub fn sampling_level(self) -> u32 {
        8 - self.allocation[0].max(self.allocation[1]).ilog2()
    }

    /// Sample one tightly packed indexed source frame and zero-pad its allocation.
    /// Rows retain the original BSI orientation. No palette conversion or filtering
    /// occurs here; callers resolve index zero through the active palette as usual.
    pub fn sample_indexed(self, source: &[u8]) -> Result<Vec<u8>> {
        let [width, height] = self.source.map(usize::from);
        if source.len() != width * height {
            return Err(Error::Parse(format!(
                "Glide texture {width}x{height}: expected {} indexed pixels, got {}",
                width * height,
                source.len()
            )));
        }
        let [aw, ah] = self.allocation.map(usize::from);
        let [sx, sy] = self.stride.map(usize::from);
        let mut pixels = vec![0; aw * ah];
        for (y, source_y) in (0..height).step_by(sy).enumerate() {
            for (x, source_x) in (0..width).step_by(sx).enumerate() {
                pixels[y * aw + x] = source[source_y * width + source_x];
            }
        }
        Ok(pixels)
    }
}
