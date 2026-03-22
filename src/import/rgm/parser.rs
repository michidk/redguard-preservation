//! RGM file parser using nom

use crate::import::rgm::{
    MpobRecord, MprpRecord, MpsRecord, MpslRecord, RaexRecord, RavcRecord, RgmFile, RgmSection,
    RgmSectionHeader,
};
use log::{debug, trace};
use nom::{
    IResult,
    bytes::complete::take,
    number::complete::{be_u32, le_i8, le_i16, le_u16, le_u32},
};

/// Parse a 4-byte section name
fn section_name(input: &[u8]) -> IResult<&[u8], [u8; 4]> {
    let (input, bytes) = take(4usize)(input)?;
    let mut name = [0u8; 4];
    name.copy_from_slice(bytes);
    Ok((input, name))
}

fn section_header(input: &[u8]) -> IResult<&[u8], RgmSectionHeader> {
    let (input, name) = section_name(input)?;
    let (input, data_length) = be_u32(input)?;

    Ok((
        input,
        RgmSectionHeader {
            name,
            data_length,
            record_count: None,
        },
    ))
}

fn le_u24(input: &[u8]) -> IResult<&[u8], u32> {
    let (input, bytes) = take(3usize)(input)?;
    let value = u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16);
    Ok((input, value))
}

fn le_i24(input: &[u8]) -> IResult<&[u8], i32> {
    let (input, value) = le_u24(input)?;
    let signed = if value & 0x0080_0000 != 0 {
        (value | 0xFF00_0000).cast_signed()
    } else {
        value.cast_signed()
    };
    Ok((input, signed))
}

fn mps_record(input: &[u8]) -> IResult<&[u8], MpsRecord> {
    let (input, id) = le_u32(input)?;
    let (input, model_name_bytes) = take(12usize)(input)?;
    let mut model_name = [0u8; 12];
    model_name.copy_from_slice(model_name_bytes);
    let (input, pos_x) = le_i24(input)?;
    let (input, _x_pad) = take(1usize)(input)?;
    let (input, pos_y) = le_i24(input)?;
    let (input, _y_pad) = take(1usize)(input)?;
    let (input, pos_z) = le_u24(input)?;
    let (input, _z_pad) = take(1usize)(input)?;

    let mut rotation_matrix = [0i32; 9];
    let mut remaining = input;
    for value in &mut rotation_matrix {
        let (next, parsed) = le_u32(remaining)?;
        *value = parsed.cast_signed();
        remaining = next;
    }

    let (input, trailing_bytes) = take(2usize)(remaining)?;
    let trailing: [u8; 2] = trailing_bytes.try_into().expect("trailing must be 2 bytes");

    Ok((
        input,
        MpsRecord {
            id,
            model_name,
            pos_x,
            pos_y,
            pos_z,
            rotation_matrix,
            trailing,
        },
    ))
}

/// Parse MPSO section data
fn mps_section_data(input: &[u8]) -> IResult<&[u8], Vec<MpsRecord>> {
    let (input, record_count) = le_u32(input)?;
    let mut records = Vec::new();
    let mut remaining = input;

    for _ in 0..record_count {
        let (input, record) = mps_record(remaining)?;
        records.push(record);
        remaining = input;
    }

    Ok((remaining, records))
}

/// Parse MPOB section data
fn mpob_section_data(input: &[u8]) -> IResult<&[u8], Vec<MpobRecord>> {
    let (input, record_count) = le_u32(input)?;
    let mut records = Vec::new();
    let mut remaining = input;

    for record_idx in 0..record_count {
        let (next, record) = mpob_record(remaining)?;
        debug!(
            "MPOB record {} script='{}' model='{}' static={} pos=({}, {}, {})",
            record_idx,
            String::from_utf8_lossy(&record.script_name).trim_matches('\0'),
            String::from_utf8_lossy(&record.model_name).trim_matches('\0'),
            record.is_static,
            record.pos_x,
            record.pos_y,
            record.pos_z
        );
        records.push(record);
        remaining = next;
    }

    debug!("Successfully parsed {} MPOB records", records.len());
    Ok((remaining, records))
}

fn mpob_record(input: &[u8]) -> IResult<&[u8], MpobRecord> {
    let (input, id) = le_u32(input)?;
    let (input, object_type) = take(1usize)(input)?;
    let (input, is_active) = take(1usize)(input)?;
    let object_type = object_type[0];
    let is_active = is_active[0];

    let (input, script_name_bytes) = take(9usize)(input)?;
    let script_name: [u8; 9] = script_name_bytes
        .try_into()
        .expect("MPOB script name must be 9 bytes");
    let (input, model_name_bytes) = take(9usize)(input)?;
    let model_name: [u8; 9] = model_name_bytes
        .try_into()
        .expect("MPOB model name must be 9 bytes");

    let (input, is_static) = take(1usize)(input)?;
    let is_static = is_static[0];
    let (input, unknown1) = le_i16(input)?;

    let (input, pos_x) = le_i24(input)?;
    let (input, x_padding_byte) = take(1usize)(input)?;
    let pad_x = x_padding_byte[0];
    let (input, pos_y) = le_i24(input)?;
    let (input, y_padding_byte) = take(1usize)(input)?;
    let pad_y = y_padding_byte[0];
    let (input, pos_z) = le_u24(input)?;

    let (input, angle_x) = le_u32(input)?;
    let (input, angle_y) = le_u32(input)?;
    let (input, angle_z) = le_u32(input)?;

    let (input, texture_data_raw) = le_i16(input)?;
    let texture_word = texture_data_raw.cast_unsigned();
    let texture_id = u8::try_from(texture_word >> 7).unwrap_or_default();
    let image_id = u8::try_from(texture_word & 0x7F).unwrap_or_default();
    let (input, intensity) = le_i16(input)?;
    let (input, radius) = le_i16(input)?;
    let (input, model_id) = le_i16(input)?;
    let (input, world_id) = le_i16(input)?;
    let (input, red) = le_i16(input)?;
    let (input, green) = le_i16(input)?;
    let (input, blue) = le_i16(input)?;

    Ok((
        input,
        MpobRecord {
            id,
            object_type,
            is_active,
            script_name,
            model_name,
            is_static,
            unknown1,
            pos_x,
            pad_x,
            pos_y,
            pad_y,
            pos_z,
            angle_x,
            angle_y,
            angle_z,
            texture_data_raw,
            texture_id,
            image_id,
            intensity,
            radius,
            model_id,
            world_id,
            red,
            green,
            blue,
        },
    ))
}

fn ravc_record(input: &[u8]) -> IResult<&[u8], RavcRecord> {
    let (input, offset_x) = le_i8(input)?;
    let (input, offset_y) = le_i8(input)?;
    let (input, offset_z) = le_i8(input)?;
    let (input, vertex) = le_u16(input)?;
    let (input, radius) = le_u32(input)?;
    Ok((
        input,
        RavcRecord {
            offset_x,
            offset_y,
            offset_z,
            vertex,
            radius,
        },
    ))
}

fn ravc_section_data(input: &[u8]) -> IResult<&[u8], Vec<RavcRecord>> {
    let mut records = Vec::new();
    let mut remaining = input;
    while remaining.len() >= 9 {
        let (next, record) = ravc_record(remaining)?;
        records.push(record);
        remaining = next;
    }
    Ok((remaining, records))
}

const RAEX_RECORD_SIZE: usize = 30;

fn raex_record(input: &[u8]) -> IResult<&[u8], RaexRecord> {
    let (input, grip0) = le_i16(input)?;
    let (input, grip1) = le_i16(input)?;
    let (input, scabbard0) = le_i16(input)?;
    let (input, scabbard1) = le_i16(input)?;
    let (input, unknown_08) = le_i16(input)?;
    let (input, texture_id) = le_i16(input)?;
    let (input, v_vertex) = le_i16(input)?;
    let (input, v_size) = le_i16(input)?;
    let (input, taunt_id) = le_i16(input)?;
    let (input, unknown_12) = le_i16(input)?;
    let (input, unknown_14) = le_i16(input)?;
    let (input, unknown_16) = le_i16(input)?;
    let (input, range_min) = le_i16(input)?;
    let (input, range_ideal) = le_i16(input)?;
    let (input, range_max) = le_i16(input)?;
    Ok((
        input,
        RaexRecord {
            grip0,
            grip1,
            scabbard0,
            scabbard1,
            unknown_08,
            texture_id,
            v_vertex,
            v_size,
            taunt_id,
            unknown_12,
            unknown_14,
            unknown_16,
            range_min,
            range_ideal,
            range_max,
        },
    ))
}

fn raex_section_data(input: &[u8]) -> IResult<&[u8], Vec<RaexRecord>> {
    let mut records = Vec::new();
    let mut remaining = input;
    while remaining.len() >= RAEX_RECORD_SIZE {
        let (next, record) = raex_record(remaining)?;
        records.push(record);
        remaining = next;
    }
    Ok((remaining, records))
}

fn mprp_record(input: &[u8]) -> IResult<&[u8], MprpRecord> {
    let (input, id) = le_u32(input)?;
    let (input, unknown0_bytes) = take(1usize)(input)?;
    let unknown0 = unknown0_bytes[0];
    let (input, pos_x) = le_i24(input)?;
    let (input, x_padding_byte) = take(1usize)(input)?;
    let pad_x = x_padding_byte[0];
    let (input, pos_y) = le_i24(input)?;
    let (input, y_padding_byte) = take(1usize)(input)?;
    let pad_y = y_padding_byte[0];
    let (input, pos_z) = le_u24(input)?;
    let (input, angle_y) = le_u32(input)?;
    let (input, rope_type) = le_u32(input)?;
    let (input, swing) = le_u32(input)?;
    let (input, speed) = le_u32(input)?;
    let (input, length) = le_i16(input)?;
    let (input, static_model_bytes) = take(9usize)(input)?;
    let (input, rope_model_bytes) = take(9usize)(input)?;

    let mut unknown1 = [0_i32; 7];
    let mut remaining = input;
    for value in &mut unknown1 {
        let (next, parsed) = le_u32(remaining)?;
        *value = parsed.cast_signed();
        remaining = next;
    }

    Ok((
        remaining,
        MprpRecord {
            id,
            unknown0,
            pos_x,
            pad_x,
            pos_y,
            pad_y,
            pos_z,
            angle_y: angle_y.cast_signed(),
            rope_type: rope_type.cast_signed(),
            swing: swing.cast_signed(),
            speed: speed.cast_signed(),
            length,
            static_model: String::from_utf8_lossy(static_model_bytes)
                .trim_matches('\0')
                .to_string(),
            rope_model: String::from_utf8_lossy(rope_model_bytes)
                .trim_matches('\0')
                .to_string(),
            unknown1,
        },
    ))
}

fn mprp_section_data(input: &[u8]) -> IResult<&[u8], Vec<MprpRecord>> {
    let (input, record_count) = le_u32(input)?;
    let mut records = Vec::new();
    let mut remaining = input;

    for _ in 0..record_count {
        let (next, record) = mprp_record(remaining)?;
        records.push(record);
        remaining = next;
    }

    Ok((remaining, records))
}

fn mpsl_record(input: &[u8]) -> IResult<&[u8], MpslRecord> {
    let (input, red_byte) = take(1usize)(input)?;
    let color_r = red_byte[0];
    let (input, green_byte) = take(1usize)(input)?;
    let color_g = green_byte[0];
    let (input, blue_byte) = take(1usize)(input)?;
    let color_b = blue_byte[0];
    let (input, color_flags_bytes) = take(1usize)(input)?;
    let color_flags = color_flags_bytes[0];
    let (input, unknown0) = le_u32(input)?;
    let (input, pos_x) = le_i24(input)?;
    let (input, _pad_x) = take(1usize)(input)?;
    let (input, pos_y) = le_i24(input)?;
    let (input, _pad_y) = take(1usize)(input)?;
    let (input, pos_z) = le_u24(input)?;
    let (input, _pad_z) = take(1usize)(input)?;
    let (input, param0) = le_i16(input)?;
    let (input, param1) = le_i16(input)?;
    let (input, unknown1_bytes) = take(18usize)(input)?;
    let mut unknown1 = [0_u8; 18];
    unknown1.copy_from_slice(unknown1_bytes);

    Ok((
        input,
        MpslRecord {
            color_r,
            color_g,
            color_b,
            color_flags,
            unknown0,
            pos_x,
            pos_y,
            pos_z,
            param0,
            param1,
            unknown1,
        },
    ))
}

fn mpsl_section_data(input: &[u8]) -> IResult<&[u8], Vec<MpslRecord>> {
    let (input, record_count) = le_u32(input)?;
    let mut records = Vec::new();
    let mut remaining = input;

    for _ in 0..record_count {
        let (next, record) = mpsl_record(remaining)?;
        records.push(record);
        remaining = next;
    }

    Ok((remaining, records))
}

fn take_payload<'a>(input: &'a [u8], header: &RgmSectionHeader) -> IResult<&'a [u8], &'a [u8]> {
    let length = usize::try_from(header.data_length).unwrap_or_default();
    take(length)(input)
}

fn raw_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
    build: impl FnOnce(RgmSectionHeader, Vec<u8>) -> RgmSection,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    Ok((input, build(*header, data.to_vec())))
}

fn parse_raex_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match raex_section_data(data) {
        Ok(([], records)) => Ok((input, RgmSection::RaexParsed(*header, records))),
        Ok((remaining, _)) => {
            debug!(
                "RAEX typed parse left {} trailing bytes, treating as raw data",
                remaining.len()
            );
            Ok((input, RgmSection::Raex(*header, data.to_vec())))
        }
        Err(error) => {
            debug!("Failed to parse RAEX records: {error:?}, treating as raw data");
            Ok((input, RgmSection::Raex(*header, data.to_vec())))
        }
    }
}

fn parse_ravc_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match ravc_section_data(data) {
        Ok(([], records)) => Ok((input, RgmSection::RavcParsed(*header, records))),
        Ok((remaining, _)) => {
            debug!(
                "RAVC typed parse left {} trailing bytes, treating as raw data",
                remaining.len()
            );
            Ok((input, RgmSection::Ravc(*header, data.to_vec())))
        }
        Err(error) => {
            debug!("Failed to parse RAVC records: {error:?}, treating as raw data");
            Ok((input, RgmSection::Ravc(*header, data.to_vec())))
        }
    }
}

fn parse_mprp_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match mprp_section_data(data) {
        Ok(([], records)) => Ok((input, RgmSection::MprpParsed(*header, records))),
        Ok((remaining, _)) => {
            debug!(
                "MPRP typed parse left {} trailing bytes, treating as raw data",
                remaining.len()
            );
            Ok((input, RgmSection::Mprp(*header, data.to_vec())))
        }
        Err(error) => {
            debug!("Failed to parse MPRP records: {error:?}, treating as raw data");
            Ok((input, RgmSection::Mprp(*header, data.to_vec())))
        }
    }
}

fn parse_mpl_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match mpsl_section_data(data) {
        Ok(([], records)) => Ok((input, RgmSection::MplParsed(*header, records))),
        Ok((remaining, _)) => {
            debug!(
                "MPSL typed parse left {} trailing bytes, treating as raw data",
                remaining.len()
            );
            Ok((input, RgmSection::Mpl(*header, data.to_vec())))
        }
        Err(error) => {
            debug!("Failed to parse MPSL records: {error:?}, treating as raw data");
            Ok((input, RgmSection::Mpl(*header, data.to_vec())))
        }
    }
}

fn parse_mpob_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match mpob_section_data(data) {
        Ok((_, records)) => {
            debug!("Successfully parsed {} MPOB records", records.len());
            Ok((input, RgmSection::MpobParsed(*header, records)))
        }
        Err(error) => {
            debug!("Failed to parse MPOB records: {error:?}, treating as raw data");
            Ok((input, RgmSection::Mpob(*header, data.to_vec())))
        }
    }
}

fn parse_mps_section<'a>(
    input: &'a [u8],
    header: &RgmSectionHeader,
) -> IResult<&'a [u8], RgmSection> {
    let (input, data) = take_payload(input, header)?;
    match mps_section_data(data) {
        Ok((_, records)) => {
            debug!("Successfully parsed {} MPSO records", records.len());
            Ok((input, RgmSection::Mps(*header, records)))
        }
        Err(error) => {
            debug!("Failed to parse MPSO records: {error:?}, returning empty record list");
            Ok((input, RgmSection::Mps(*header, Vec::new())))
        }
    }
}

/// Parse section data based on section type
fn section_data<'a>(input: &'a [u8], header: &RgmSectionHeader) -> IResult<&'a [u8], RgmSection> {
    let section_name = String::from_utf8_lossy(&header.name);

    match section_name.as_ref() {
        "RAFS" => raw_section(input, header, RgmSection::Rafs),
        "RAST" => raw_section(input, header, RgmSection::Rast),
        "RASB" => raw_section(input, header, RgmSection::Rasb),
        "RAVA" => raw_section(input, header, RgmSection::Rava),
        "RASC" => raw_section(input, header, RgmSection::Rasc),
        "RAHK" => raw_section(input, header, RgmSection::Rahk),
        "RALC" => raw_section(input, header, RgmSection::Ralc),
        "RAEX" => parse_raex_section(input, header),
        "RAAT" => raw_section(input, header, RgmSection::Raat),
        "RAAN" => raw_section(input, header, RgmSection::Raan),
        "RAGR" => raw_section(input, header, RgmSection::Ragr),
        "RANM" => raw_section(input, header, RgmSection::Ranm),
        "RAVC" => parse_ravc_section(input, header),
        "MPOB" => parse_mpob_section(input, header),
        "MPRP" => parse_mprp_section(input, header),
        "MPSO" => parse_mps_section(input, header),
        "MPL " | "MPSL" => parse_mpl_section(input, header),
        "MPF " | "MPSF" => raw_section(input, header, RgmSection::Mpf),
        "MPM " | "MPMK" => raw_section(input, header, RgmSection::Mpm),
        "MPSZ" => raw_section(input, header, RgmSection::Mpsz),
        "WDNM" => raw_section(input, header, RgmSection::Wdnm),
        "FLAT" => raw_section(input, header, RgmSection::Flat),
        "END " => Ok((input, RgmSection::End(*header))),
        _ => raw_section(input, header, RgmSection::Rahd),
    }
}

/// Parse a complete RGM file
#[allow(clippy::missing_errors_doc)] // nom-based parser surface is internal and error details are represented in the nom type.
pub fn parse_rgm_file(input: &[u8]) -> IResult<&[u8], RgmFile> {
    let mut sections = Vec::new();
    let mut remaining = input;

    while remaining.len() >= 8 {
        match section_header(remaining) {
            Ok((input, header)) => {
                if header.name == [b'E', b'N', b'D', b' '] {
                    sections.push(RgmSection::End(header));
                    remaining = input;
                    break;
                }
                if input.len() < header.data_length as usize {
                    trace!(
                        "Not enough data for section (need {}, have {})",
                        header.data_length,
                        input.len()
                    );
                    break;
                }
                match section_data(input, &header) {
                    Ok((input, section)) => {
                        sections.push(section);
                        remaining = input;
                    }
                    Err(e) => {
                        trace!("Failed to parse section data: {e:?}");
                        break;
                    }
                }
            }
            Err(e) => {
                trace!("Failed to parse section header: {e:?}");
                break;
            }
        }
    }
    debug!("Parsed {} sections", sections.len());
    Ok((remaining, RgmFile { sections }))
}
