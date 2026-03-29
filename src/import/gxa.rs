use crate::{Result, error::Error};

const LZHUF_N: usize = 4096;
const LZHUF_F: usize = 60;
const LZHUF_THRESHOLD: usize = 2;
const LZHUF_N_CHAR: usize = 256 + LZHUF_F - LZHUF_THRESHOLD;
const LZHUF_T: usize = LZHUF_N_CHAR * 2 - 1;
const LZHUF_R: usize = LZHUF_T - 1;
const LZHUF_MAX_FREQ: u16 = 0x8000;

const LZHUF_D_CODE: [u8; 256] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x09,
    0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
    0x0C, 0x0C, 0x0C, 0x0C, 0x0D, 0x0D, 0x0D, 0x0D, 0x0E, 0x0E, 0x0E, 0x0E, 0x0F, 0x0F, 0x0F, 0x0F,
    0x10, 0x10, 0x10, 0x10, 0x11, 0x11, 0x11, 0x11, 0x12, 0x12, 0x12, 0x12, 0x13, 0x13, 0x13, 0x13,
    0x14, 0x14, 0x14, 0x14, 0x15, 0x15, 0x15, 0x15, 0x16, 0x16, 0x16, 0x16, 0x17, 0x17, 0x17, 0x17,
    0x18, 0x18, 0x19, 0x19, 0x1A, 0x1A, 0x1B, 0x1B, 0x1C, 0x1C, 0x1D, 0x1D, 0x1E, 0x1E, 0x1F, 0x1F,
    0x20, 0x20, 0x21, 0x21, 0x22, 0x22, 0x23, 0x23, 0x24, 0x24, 0x25, 0x25, 0x26, 0x26, 0x27, 0x27,
    0x28, 0x28, 0x29, 0x29, 0x2A, 0x2A, 0x2B, 0x2B, 0x2C, 0x2C, 0x2D, 0x2D, 0x2E, 0x2E, 0x2F, 0x2F,
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
];

const LZHUF_D_LEN: [u8; 256] = [
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04,
    0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
];

#[derive(Debug, Clone)]
pub struct GxaFrame {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct GxaFile {
    pub title: String,
    pub frame_count: u16,
    pub frames: Vec<GxaFrame>,
}

fn read_i16_le(data: &[u8], offset: usize) -> Result<i16> {
    if offset + 2 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_i16 out of bounds at offset {offset}"
        )));
    }
    Ok(i16::from_le_bytes([data[offset], data[offset + 1]]))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_u32 out of bounds at offset {offset}"
        )));
    }
    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::Parse(format!(
            "GXA read_u32 out of bounds at offset {offset}"
        )));
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn decode_frame_rgba(
    width: usize,
    height: usize,
    pixels: &[u8],
    palette: &[[u8; 3]; 256],
) -> Result<Vec<u8>> {
    let pixel_count = width
        .checked_mul(height)
        .ok_or_else(|| Error::Parse("GXA width*height overflow".to_string()))?;
    if pixels.len() < pixel_count {
        return Err(Error::Parse(format!(
            "GXA frame pixel data too small: got {}, need {pixel_count}",
            pixels.len()
        )));
    }

    let mut rgba = vec![0u8; pixel_count * 4];
    for y in 1..=height {
        for x in 0..width {
            let src = x + (y - 1) * width;
            let dst = x + (height - y) * width;
            let idx = usize::from(pixels[src]);
            let o = dst * 4;
            if idx == 0 {
                rgba[o..o + 4].copy_from_slice(&[0, 0, 0, 0]);
            } else {
                let c = palette[idx];
                rgba[o..o + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }

    Ok(rgba)
}

fn decompress_rle(compressed: &[u8], pixel_count: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(pixel_count);
    let mut cur = 0usize;

    while out.len() < pixel_count {
        if cur >= compressed.len() {
            return Err(Error::Parse(
                "GXA RLE compressed stream truncated".to_string(),
            ));
        }
        let ctrl = compressed[cur];
        cur += 1;

        if ctrl & 0x80 != 0 {
            if cur >= compressed.len() {
                return Err(Error::Parse(
                    "GXA RLE repeat run missing value byte".to_string(),
                ));
            }
            let value = compressed[cur];
            cur += 1;
            let run_len = usize::from(ctrl & 0x7F) + 1;
            for _ in 0..run_len {
                if out.len() == pixel_count {
                    break;
                }
                out.push(value);
            }
        } else {
            let run_len = usize::from(ctrl) + 1;
            if cur + run_len > compressed.len() {
                return Err(Error::Parse(
                    "GXA RLE literal run exceeds compressed stream".to_string(),
                ));
            }
            let remaining = pixel_count - out.len();
            let copy_len = run_len.min(remaining);
            out.extend_from_slice(&compressed[cur..cur + copy_len]);
            cur += run_len;
        }
    }

    Ok(out)
}

struct LzhufDecoder<'a> {
    input: &'a [u8],
    input_pos: usize,
    getbuf: u16,
    getlen: u8,
    freq: [u16; LZHUF_T + 1],
    prnt: [i16; LZHUF_T + LZHUF_N_CHAR],
    son: [i16; LZHUF_T],
    ring: [u8; LZHUF_N],
    r: usize,
}

impl<'a> LzhufDecoder<'a> {
    fn new(input: &'a [u8]) -> Self {
        let mut decoder = Self {
            input,
            input_pos: 0,
            getbuf: 0,
            getlen: 0,
            freq: [0; LZHUF_T + 1],
            prnt: [0; LZHUF_T + LZHUF_N_CHAR],
            son: [0; LZHUF_T],
            ring: [0x20; LZHUF_N],
            r: LZHUF_N - LZHUF_F,
        };
        decoder.start_huff();
        decoder
    }

    fn next_input_byte(&mut self) -> u8 {
        if self.input_pos < self.input.len() {
            let b = self.input[self.input_pos];
            self.input_pos += 1;
            b
        } else {
            0
        }
    }

    fn get_bit(&mut self) -> u16 {
        while self.getlen <= 8 {
            let b = u16::from(self.next_input_byte());
            self.getbuf |= b << (8 - self.getlen);
            self.getlen += 8;
        }
        let bit = self.getbuf >> 15;
        self.getbuf <<= 1;
        self.getlen -= 1;
        bit
    }

    fn get_byte(&mut self) -> u8 {
        while self.getlen <= 8 {
            let b = u16::from(self.next_input_byte());
            self.getbuf |= b << (8 - self.getlen);
            self.getlen += 8;
        }
        let byte = ((self.getbuf >> 8) & 0xFF) as u8;
        self.getbuf <<= 8;
        self.getlen -= 8;
        byte
    }

    fn start_huff(&mut self) {
        for i in 0..LZHUF_N_CHAR {
            self.freq[i] = 1;
            self.son[i] = (i + LZHUF_T) as i16;
            self.prnt[i + LZHUF_T] = i as i16;
        }

        let mut i = 0usize;
        let mut j = LZHUF_N_CHAR;
        while j <= LZHUF_R {
            self.freq[j] = self.freq[i] + self.freq[i + 1];
            self.son[j] = i as i16;
            self.prnt[i] = j as i16;
            self.prnt[i + 1] = j as i16;
            i += 2;
            j += 1;
        }

        self.freq[LZHUF_T] = u16::MAX;
        self.prnt[LZHUF_R] = 0;
    }

    fn reconst(&mut self) {
        let mut j = 0usize;
        for i in 0..LZHUF_T {
            if usize::try_from(self.son[i]).is_ok_and(|v| v >= LZHUF_T) {
                self.freq[j] = self.freq[i].div_ceil(2);
                self.son[j] = self.son[i];
                j += 1;
            }
        }

        let mut i = 0usize;
        let mut node = LZHUF_N_CHAR;
        while node < LZHUF_T {
            let k = i + 1;
            let f = self.freq[i] + self.freq[k];
            self.freq[node] = f;

            let mut insert = node;
            while insert > 0 && f < self.freq[insert - 1] {
                insert -= 1;
            }

            if insert < node {
                self.freq.copy_within(insert..node, insert + 1);
                self.son.copy_within(insert..node, insert + 1);
            }

            self.freq[insert] = f;
            self.son[insert] = i as i16;

            i += 2;
            node += 1;
        }

        for i in 0..LZHUF_T {
            let child = usize::try_from(self.son[i]).unwrap_or(0);
            if child >= LZHUF_T {
                self.prnt[child] = i as i16;
            } else {
                self.prnt[child] = i as i16;
                self.prnt[child + 1] = i as i16;
            }
        }

        self.freq[LZHUF_T] = u16::MAX;
        self.prnt[LZHUF_R] = 0;
    }

    fn update(&mut self, mut c: usize) {
        if self.freq[LZHUF_R] == LZHUF_MAX_FREQ {
            self.reconst();
        }

        c = usize::try_from(self.prnt[c]).unwrap_or(0);
        loop {
            let updated = self.freq[c] + 1;
            self.freq[c] = updated;

            let mut l = c + 1;
            if updated > self.freq[l] {
                while updated > self.freq[l + 1] {
                    l += 1;
                }

                self.freq[c] = self.freq[l];
                self.freq[l] = updated;

                let i = usize::try_from(self.son[c]).unwrap_or(0);
                self.prnt[i] = l as i16;
                if i < LZHUF_T {
                    self.prnt[i + 1] = l as i16;
                }

                let j = usize::try_from(self.son[l]).unwrap_or(0);
                self.son[l] = i as i16;
                self.prnt[j] = c as i16;
                if j < LZHUF_T {
                    self.prnt[j + 1] = c as i16;
                }

                self.son[c] = j as i16;
                c = l;
            }

            c = usize::try_from(self.prnt[c]).unwrap_or(0);
            if c == 0 {
                break;
            }
        }
    }

    fn decode_char(&mut self) -> usize {
        let mut c = usize::try_from(self.son[LZHUF_R]).unwrap_or(0);
        while c < LZHUF_T {
            let bit = self.get_bit() as usize;
            c = usize::try_from(self.son[c + bit]).unwrap_or(0);
        }
        self.update(c);
        c - LZHUF_T
    }

    fn decode_position(&mut self) -> usize {
        let mut i = u16::from(self.get_byte());
        let c = usize::from(LZHUF_D_CODE[usize::from(i)]) << 6;
        let mut j = i32::from(LZHUF_D_LEN[usize::from(i)]) - 2;
        while j > 0 {
            i = (i << 1) | self.get_bit();
            j -= 1;
        }
        c | usize::from(i & 0x3F)
    }

    fn decompress(&mut self, pixel_count: usize) -> Vec<u8> {
        let mut output = Vec::with_capacity(pixel_count);

        while output.len() < pixel_count {
            let c = self.decode_char();
            if c < 256 {
                let ch = c as u8;
                output.push(ch);
                self.ring[self.r] = ch;
                self.r = (self.r + 1) & (LZHUF_N - 1);
            } else {
                let length = c - 255 + LZHUF_THRESHOLD;
                let pos = self.decode_position();
                let src = self.r.wrapping_sub(pos).wrapping_sub(1) & (LZHUF_N - 1);
                for k in 0..length {
                    if output.len() == pixel_count {
                        break;
                    }
                    let ch = self.ring[(src + k) & (LZHUF_N - 1)];
                    output.push(ch);
                    self.ring[self.r] = ch;
                    self.r = (self.r + 1) & (LZHUF_N - 1);
                }
            }
        }

        output
    }
}

fn decompress_lzhuf(compressed: &[u8], pixel_count: usize) -> Result<Vec<u8>> {
    let mut decoder = LzhufDecoder::new(compressed);
    Ok(decoder.decompress(pixel_count))
}

pub fn parse_gxa_file(data: &[u8]) -> Result<GxaFile> {
    let mut cursor = 0usize;

    let mut title = String::new();
    let mut frame_count = 0u16;
    let mut palette = [[0u8; 3]; 256];
    let mut has_palette = false;
    let mut decoded_frames: Vec<GxaFrame> = Vec::new();

    while cursor + 4 <= data.len() {
        let tag_bytes: [u8; 4] = data[cursor..cursor + 4]
            .try_into()
            .map_err(|_| Error::Parse("GXA section tag read failed".to_string()))?;
        cursor += 4;

        if tag_bytes == *b"END " {
            break;
        }

        let section_len = usize::try_from(read_u32_be(data, cursor)?)
            .map_err(|_| Error::Parse("GXA section length does not fit usize".to_string()))?;
        cursor += 4;

        let section_end = if cursor + section_len > data.len() {
            if tag_bytes == *b"BBMP" {
                data.len()
            } else {
                return Err(Error::Parse(format!(
                    "GXA section out of bounds at offset {cursor} with length {section_len}"
                )));
            }
        } else {
            cursor + section_len
        };

        let section = &data[cursor..section_end];
        cursor = section_end;

        match &tag_bytes {
            b"BMHD" => {
                if section.len() < 34 {
                    return Err(Error::Parse(format!(
                        "GXA BMHD too small: {} bytes (need >= 34)",
                        section.len()
                    )));
                }
                let raw_title = &section[..22];
                let end = raw_title
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(raw_title.len());
                title = String::from_utf8_lossy(&raw_title[..end]).to_string();
                let count = read_i16_le(section, 32)?;
                if count < 0 {
                    return Err(Error::Parse(format!(
                        "GXA BMHD negative frame count: {count}"
                    )));
                }
                frame_count = u16::try_from(count)
                    .map_err(|_| Error::Parse("GXA frame count conversion failed".to_string()))?;
            }
            b"BPAL" => {
                if section.len() < 256 * 3 {
                    return Err(Error::Parse(format!(
                        "GXA BPAL too small: {} bytes (need >= 768)",
                        section.len()
                    )));
                }
                for (i, color) in palette.iter_mut().enumerate() {
                    let off = i * 3;
                    *color = [section[off], section[off + 1], section[off + 2]];
                }
                has_palette = true;
            }
            b"BBMP" => {
                if frame_count == 0 {
                    return Err(Error::Parse(
                        "GXA BBMP encountered before BMHD with valid frame count".to_string(),
                    ));
                }
                if !has_palette {
                    return Err(Error::Parse(
                        "GXA BBMP encountered before BPAL palette".to_string(),
                    ));
                }

                let mut sec_cur = 0usize;
                decoded_frames.clear();
                decoded_frames.reserve(usize::from(frame_count));

                for frame_idx in 0..usize::from(frame_count) {
                    if sec_cur + 18 > section.len() {
                        return Err(Error::Parse(format!(
                            "GXA BBMP frame {frame_idx} header out of bounds"
                        )));
                    }

                    let width_i16 = read_i16_le(section, sec_cur + 2)?;
                    let height_i16 = read_i16_le(section, sec_cur + 4)?;
                    if width_i16 <= 0 || height_i16 <= 0 {
                        return Err(Error::Parse(format!(
                            "GXA BBMP invalid dimensions for frame {frame_idx}: {width_i16}x{height_i16}"
                        )));
                    }
                    let width = usize::try_from(width_i16)
                        .map_err(|_| Error::Parse("GXA width conversion failed".to_string()))?;
                    let height = usize::try_from(height_i16)
                        .map_err(|_| Error::Parse("GXA height conversion failed".to_string()))?;

                    let pixel_count = width.checked_mul(height).ok_or_else(|| {
                        Error::Parse(format!(
                            "GXA BBMP frame {frame_idx} dimensions overflow: {width}x{height}"
                        ))
                    })?;

                    let compression = read_i16_le(section, sec_cur + 10)?;

                    let pixels = match compression {
                        0 => {
                            let data_start = sec_cur + 18;
                            let data_end = data_start + pixel_count;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} raw payload out of bounds"
                                )));
                            }
                            sec_cur = data_end;
                            section[data_start..data_end].to_vec()
                        }
                        1 => {
                            if sec_cur + 22 > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} RLE header out of bounds"
                                )));
                            }
                            let comp_size = usize::try_from(read_u32_le(section, sec_cur + 18)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA RLE compressed size conversion failed".to_string(),
                                    )
                                })?;
                            let data_start = sec_cur + 22;
                            let data_end = data_start + comp_size;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} RLE payload out of bounds"
                                )));
                            }
                            let decompressed =
                                decompress_rle(&section[data_start..data_end], pixel_count)?;
                            sec_cur = data_end;
                            decompressed
                        }
                        2 => {
                            if sec_cur + 26 > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} LZHUF header out of bounds"
                                )));
                            }
                            let comp_size = usize::try_from(read_u32_le(section, sec_cur + 18)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA LZHUF compressed size conversion failed".to_string(),
                                    )
                                })?;
                            let _uncomp_size = usize::try_from(read_u32_le(section, sec_cur + 22)?)
                                .map_err(|_| {
                                    Error::Parse(
                                        "GXA LZHUF uncompressed size conversion failed".to_string(),
                                    )
                                })?;
                            let data_start = sec_cur + 26;
                            let data_end = data_start + comp_size;
                            if data_end > section.len() {
                                return Err(Error::Parse(format!(
                                    "GXA BBMP frame {frame_idx} LZHUF payload out of bounds"
                                )));
                            }
                            let decompressed =
                                decompress_lzhuf(&section[data_start..data_end], pixel_count)?;
                            sec_cur = data_end;
                            decompressed
                        }
                        _ => {
                            return Err(Error::Parse(format!(
                                "GXA BBMP frame {frame_idx} unsupported compression type {compression}"
                            )));
                        }
                    };

                    let rgba = decode_frame_rgba(width, height, &pixels, &palette)?;
                    decoded_frames.push(GxaFrame {
                        width: u16::try_from(width)
                            .map_err(|_| Error::Parse("GXA width > u16::MAX".to_string()))?,
                        height: u16::try_from(height)
                            .map_err(|_| Error::Parse("GXA height > u16::MAX".to_string()))?,
                        rgba,
                    });
                }
            }
            _ => {}
        }
    }

    if decoded_frames.is_empty() {
        return Err(Error::Parse("GXA contains no decoded frames".to_string()));
    }

    Ok(GxaFile {
        title,
        frame_count,
        frames: decoded_frames,
    })
}
