/// Parsed 256-color palette from a Redguard COL file.
pub struct Palette {
    pub colors: [[u8; 3]; 256],
}

impl Palette {
    /// Matches a six-bit VGA request using the engine's palette search.
    /// `colors` must contain six-bit components. Index zero is excluded;
    /// exact matches win first, approximate ties win last. See startup rules.
    #[must_use]
    pub fn match_vga(&self, request: [u8; 3]) -> u8 {
        let request = request.map(|value| value & 63);
        let distance = |a: u8, b: u8| {
            let difference = i32::from(a) - i32::from(b);
            if difference < 0 {
                !difference
            } else {
                difference
            }
        };
        let luminance = |rgb: [u8; 3]| {
            i32::from(rgb[0]) * 19661 + i32::from(rgb[1]) * 38666 + i32::from(rgb[2]) * 7209
        };
        let mut near = (9999, 0);
        let mut fallback = (999, 231);
        for (index, &color) in self.colors.iter().enumerate().skip(1) {
            if color == request {
                return index as u8;
            }
            if color.iter().map(|&c| u16::from(c)).sum::<u16>() == 189 {
                continue;
            }
            let differences = std::array::from_fn::<_, 3, _>(|i| distance(request[i], color[i]));
            let total = differences.iter().sum();
            if total <= fallback.0 {
                fallback = (total, index as u8);
            }
            if differences.iter().all(|&difference| difference <= 8) {
                let difference = (luminance(request) - luminance(color)).abs();
                if difference <= near.0 {
                    near = (difference, index as u8);
                }
            }
        }
        if near.0 <= 24 { near.1 } else { fallback.1 }
    }

    /// Parses palette bytes into a 256-entry RGB table.
    pub fn parse(data: &[u8]) -> crate::Result<Self> {
        if data.len() < 776 {
            return Err(crate::error::Error::Parse(format!(
                "COL file too small: {} bytes (expected 776)",
                data.len()
            )));
        }

        let mut colors = [[0u8; 3]; 256];
        for (i, color) in colors.iter_mut().enumerate() {
            let offset = 8 + i * 3;
            color[0] = data[offset];
            color[1] = data[offset + 1];
            color[2] = data[offset + 2];
        }

        Ok(Self { colors })
    }

    /// Returns a palette entry normalized to 0.0..=1.0 RGB values.
    #[must_use]
    pub fn get_rgb_f32(&self, index: u8) -> [f32; 3] {
        let c = self.colors[usize::from(index)];
        [
            f32::from(c[0]) / 255.0,
            f32::from(c[1]) / 255.0,
            f32::from(c[2]) / 255.0,
        ]
    }
}
