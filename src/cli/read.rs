use super::filetype::FileTypeCliExt;
use super::utils::resolve_filetype;
use crate::opts::ReadArgs;
use color_eyre::Result;
use log::info;

#[allow(clippy::needless_pass_by_value)]
pub fn handle_read_command(args: ReadArgs) -> Result<()> {
    let file_path = &args.file;
    let filetype = resolve_filetype(file_path)?;

    info!("Reading file: {}", file_path.display());
    info!("File type: {filetype:?}");

    let file_content = std::fs::read(file_path)?;
    filetype.print_read_output(&file_content)
}
