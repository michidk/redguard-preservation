use super::{Error, MpobRecord, MpsRecord, Result, RgmSection, RgmSectionHeader};

impl RgmSectionHeader {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }
}

impl MpsRecord {
    pub fn model_name(&self) -> String {
        String::from_utf8_lossy(&self.model_name)
            .trim_matches('\0')
            .to_string()
    }
}

impl MpobRecord {
    pub fn model_name(&self) -> String {
        String::from_utf8_lossy(&self.model_name)
            .trim_matches('\0')
            .to_string()
    }

    pub fn script_name(&self) -> String {
        String::from_utf8_lossy(&self.script_name)
            .trim_matches('\0')
            .to_string()
    }
}

impl RgmSection {
    pub fn header(&self) -> &RgmSectionHeader {
        match self {
            RgmSection::Rahd(header, _)
            | RgmSection::Rafs(header, _)
            | RgmSection::Rast(header, _)
            | RgmSection::Rasb(header, _)
            | RgmSection::Rava(header, _)
            | RgmSection::Rasc(header, _)
            | RgmSection::Rahk(header, _)
            | RgmSection::Ralc(header, _)
            | RgmSection::Raex(header, _)
            | RgmSection::RaexParsed(header, _)
            | RgmSection::Raat(header, _)
            | RgmSection::Raan(header, _)
            | RgmSection::Ragr(header, _)
            | RgmSection::Ranm(header, _)
            | RgmSection::Ravc(header, _)
            | RgmSection::RavcParsed(header, _)
            | RgmSection::Mpob(header, _)
            | RgmSection::MpobParsed(header, _)
            | RgmSection::Mprp(header, _)
            | RgmSection::MprpParsed(header, _)
            | RgmSection::Mps(header, _)
            | RgmSection::Mpl(header, _)
            | RgmSection::MplParsed(header, _)
            | RgmSection::Mpf(header, _)
            | RgmSection::Mpm(header, _)
            | RgmSection::Mpsz(header, _)
            | RgmSection::Wdnm(header, _)
            | RgmSection::Flat(header, _)
            | RgmSection::End(header) => header,
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
    output.push_str(&format!("Found {} sections\n", rgm_file.sections.len()));
    for (i, section) in rgm_file.sections.iter().enumerate() {
        output.push_str(&format!("Section {}: {:?}\n", i + 1, section));
        match section {
            RgmSection::Mps(_, mps_records) => {
                output.push_str(&format!("  MPSO ({} records)\n", mps_records.len()));
                for (j, record) in mps_records.iter().enumerate() {
                    if j < 5 {
                        output.push_str(&format!(
                            "    Record {}: Model='{}'\n",
                            j + 1,
                            record.model_name()
                        ));
                    } else if j == 5 {
                        output.push_str(&format!(
                            "    ... and {} more records\n",
                            mps_records.len() - 5
                        ));
                        break;
                    }
                }
            }
            RgmSection::End(header) => output.push_str(&format!("  END ({})\n", header.name())),
            _ => {}
        }
    }

    Ok(output)
}
