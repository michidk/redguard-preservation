use super::{Error, MpobRecord, MpsRecord, Result, RgmSection, RgmSectionHeader};
use std::fmt::Write;

impl RgmSectionHeader {
    #[must_use]
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }
}

impl MpsRecord {
    #[must_use]
    pub fn model_name(&self) -> String {
        String::from_utf8_lossy(&self.model_name)
            .trim_matches('\0')
            .to_string()
    }
}

impl MpobRecord {
    #[must_use]
    pub fn model_name(&self) -> String {
        String::from_utf8_lossy(&self.model_name)
            .trim_matches('\0')
            .to_string()
    }

    #[must_use]
    pub fn script_name(&self) -> String {
        String::from_utf8_lossy(&self.script_name)
            .trim_matches('\0')
            .to_string()
    }
}

impl RgmSection {
    #[must_use]
    pub const fn header(&self) -> &RgmSectionHeader {
        match self {
            Self::Rahd(header, _)
            | Self::Rafs(header, _)
            | Self::Rast(header, _)
            | Self::Rasb(header, _)
            | Self::Rava(header, _)
            | Self::Rasc(header, _)
            | Self::Rahk(header, _)
            | Self::Ralc(header, _)
            | Self::Raex(header, _)
            | Self::RaexParsed(header, _)
            | Self::Raat(header, _)
            | Self::Raan(header, _)
            | Self::Ragr(header, _)
            | Self::Ranm(header, _)
            | Self::Ravc(header, _)
            | Self::RavcParsed(header, _)
            | Self::Mpob(header, _)
            | Self::MpobParsed(header, _)
            | Self::Mprp(header, _)
            | Self::MprpParsed(header, _)
            | Self::Mps(header, _)
            | Self::Mpl(header, _)
            | Self::MplParsed(header, _)
            | Self::Mpf(header, _)
            | Self::Mpm(header, _)
            | Self::Mpsz(header, _)
            | Self::Wdnm(header, _)
            | Self::Flat(header, _)
            | Self::End(header) => header,
        }
    }
}

pub(super) fn read_i32_le(bytes: &[u8], offset: usize) -> Option<i32> {
    let chunk: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(i32::from_le_bytes(chunk))
}

pub(super) fn read_i16_le(bytes: &[u8], offset: usize) -> Option<i16> {
    let chunk: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(i16::from_le_bytes(chunk))
}

pub(super) fn read_script_name_9(bytes: &[u8], offset: usize) -> Option<String> {
    let raw = bytes.get(offset..offset + 9)?;
    Some(String::from_utf8_lossy(raw).trim_matches('\0').to_string())
}

pub(super) fn dump_rgm_impl(input: &[u8]) -> Result<String> {
    let (_, rgm_file) = super::parser::parse_rgm_file(input)
        .map_err(|e| Error::Parse(format!("Failed to parse RGM file: {e}")))?;

    let mut output = String::new();
    output.push_str("RGM File Structure:\n");
    output.push_str("==================\n\n");
    let _ = writeln!(output, "Found {} sections", rgm_file.sections.len());
    for (section_index, section) in rgm_file.sections.iter().enumerate() {
        let _ = writeln!(output, "Section {}: {:?}", section_index + 1, section);
        match section {
            RgmSection::Mps(_, mps_records) => {
                let _ = writeln!(output, "  MPSO ({} records)", mps_records.len());
                for (record_index, record) in mps_records.iter().enumerate() {
                    if record_index < 5 {
                        let _ = writeln!(
                            output,
                            "    Record {}: Model='{}'",
                            record_index + 1,
                            record.model_name()
                        );
                    } else if record_index == 5 {
                        let _ =
                            writeln!(output, "    ... and {} more records", mps_records.len() - 5);
                        break;
                    }
                }
            }
            RgmSection::End(header) => {
                let _ = writeln!(output, "  END ({})", header.name());
            }
            _ => {}
        }
    }

    Ok(output)
}
