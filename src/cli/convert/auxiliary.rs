use super::ensure_parent_dir;
use color_eyre::{Result, eyre::bail};
use log::info;
use rgpre::import::{FileType, more, opl, pif, rst, sidewinder};
use std::path::Path;

pub(crate) fn handle_auxiliary_convert(
    file: &Path,
    output: &Path,
    filetype: FileType,
) -> Result<()> {
    let bytes = std::fs::read(file)?;
    let json = match filetype {
        FileType::Pif => serde_json::to_string_pretty(&pif::parse_pif_file(&bytes)?)?,
        FileType::Opl | FileType::Ad => {
            serde_json::to_string_pretty(&opl::parse_opl_file(&bytes)?)?
        }
        FileType::Rst => serde_json::to_string_pretty(&rst::parse_rst_file(&bytes)?)?,
        FileType::Swg | FileType::Swx => {
            serde_json::to_string_pretty(&sidewinder::parse_sidewinder_file(&bytes)?)?
        }
        FileType::More => serde_json::to_string_pretty(&more::parse_more_file(&bytes)?)?,
        _ => bail!("Unsupported auxiliary format: {filetype:?}"),
    };
    ensure_parent_dir(output)?;
    std::fs::write(output, json)?;
    info!("Exported {}: {}", filetype.display_name(), output.display());
    Ok(())
}
