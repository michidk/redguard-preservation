/// Parsed 256-color palette from a Redguard COL file.
pub struct Palette {
    pub colors: [[u8; 3]; 256],
}

impl Palette {
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

        Ok(Palette { colors })
    }

    /// Returns a palette entry normalized to 0.0..=1.0 RGB values.
    pub fn get_rgb_f32(&self, index: u8) -> [f32; 3] {
        let c = self.colors[index as usize];
        [
            c[0] as f32 / 255.0,
            c[1] as f32 / 255.0,
            c[2] as f32 / 255.0,
        ]
    }
}
