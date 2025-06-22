//! Command-line interface for Redguard Preservation
//!
//! This binary provides a command-line interface for parsing ROB and 3D model files.

use clap::Parser;
use color_eyre::Result;
use log::{error, info, trace};
use std::io::Write;
mod opts;
use opts::{Commands, FileType, Opts};
use redguard_preservation::{parse_rob_with_models, parser::parse_3d_file};

fn main() -> Result<()> {
    color_eyre::install()?;

    let opts = Opts::parse();
    let opts_dbg = format!("{opts:#?}");

    env_logger::Builder::from_default_env()
        .filter_level(opts.verbose.log_level_filter())
        .format(move |buf, record| log_format(buf, record, opts.verbose.log_level_filter()))
        .init();

    trace!("Parsed Opts:\n{}", opts_dbg);

    match opts.command {
        Commands::Read { args } => {
            let file_path = &args.file;
            let filetype = args.filetype.unwrap_or_else(|| {
                FileType::from_extension(file_path).unwrap_or_else(|| {
                    error!("Could not infer file type from extension. Please specify --filetype");
                    std::process::exit(1);
                })
            });

            info!("Reading file: {}", file_path.display());
            info!("File type: {:?}", filetype);

            let file_content = std::fs::read(file_path).map_err(|e| {
                color_eyre::eyre::eyre!("Failed to read file '{}': {}", file_path.display(), e)
            })?;

            match filetype {
                FileType::Rob => {
                    match parse_rob_with_models(&file_content) {
                        Ok((rob_file, models)) => {
                            info!("Successfully parsed ROB file!");
                            info!("Header: {:?}", rob_file.header);
                            info!("Number of segments: {}", rob_file.segments.len());
                            info!("Number of embedded 3D models: {}", models.len());

                            // Print segment information
                            for (i, segment) in rob_file.segments.iter().enumerate() {
                                let name = segment.name();

                                if segment.points_to_external_file() {
                                    info!("Segment {}: '{}' points to external 3DC file", i, name);
                                } else if segment.has_embedded_3d_data() {
                                    info!(
                                        "Segment {}: '{}' embeds 3D data (size: {})",
                                        i, name, segment.size
                                    );
                                } else {
                                    info!(
                                        "Segment {}: '{}' contains other data (size: {})",
                                        i, name, segment.size
                                    );
                                }
                            }

                            // Print 3D model information
                            for (i, model) in models.iter().enumerate() {
                                info!("\n3D Model {}:", i + 1);
                                info!("  Version: {}", model.header.version_string());
                                info!("  Vertices: {}", model.header.num_vertices);
                                info!("  Faces: {}", model.header.num_faces);
                                info!("  Total face vertices: {}", model.total_face_vertices());
                                info!("  UV coordinates: {}", model.uv_coords.len());

                                if let Some((min, max)) = model.bounding_box() {
                                    info!("  Bounding box:");
                                    info!("    Min: ({:.2}, {:.2}, {:.2})", min.x, min.y, min.z);
                                    info!("    Max: ({:.2}, {:.2}, {:.2})", max.x, max.y, max.z);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse ROB file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                FileType::Model3D => match parse_3d_file(&file_content) {
                    Ok(model) => {
                        info!("Successfully parsed 3D model file!");
                        info!("Version: {}", model.header.version_string());
                        info!("Vertices: {}", model.header.num_vertices);
                        info!("Faces: {}", model.header.num_faces);
                        info!("Total face vertices: {}", model.total_face_vertices());
                        info!("UV coordinates: {}", model.uv_coords.len());

                        if let Some((min, max)) = model.bounding_box() {
                            info!("Bounding box:");
                            info!("  Min: ({:.2}, {:.2}, {:.2})", min.x, min.y, min.z);
                            info!("  Max: ({:.2}, {:.2}, {:.2})", max.x, max.y, max.z);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse 3D model file: {}", e);
                        std::process::exit(1);
                    }
                },
            }
        }
    }

    Ok(())
}

/// Formats the log messages in a minimalistic way, since we don't have a lot of output.
fn log_format(
    buf: &mut env_logger::fmt::Formatter,
    record: &log::Record,
    filter: log::LevelFilter,
) -> std::io::Result<()> {
    let level = record.level();
    let level_char = match level {
        log::Level::Trace => 'T',
        log::Level::Debug => 'D',
        log::Level::Info => 'I',
        log::Level::Warn => 'W',
        log::Level::Error => 'E',
    };
    // color using shell escape codes
    let colored_level = match level {
        log::Level::Trace => format!("\x1b[37m{level_char}\x1b[0m"),
        log::Level::Debug => format!("\x1b[36m{level_char}\x1b[0m"),
        log::Level::Info => format!("\x1b[32m{level_char}\x1b[0m"),
        log::Level::Warn => format!("\x1b[33m{level_char}\x1b[0m"),
        log::Level::Error => format!("\x1b[31m{level_char}\x1b[0m"),
    };

    // Default behavior (for info messages): only print message
    // but if level is not info and filter is set, prefix it with the colored level
    if level == log::Level::Info && filter == log::LevelFilter::Info {
        writeln!(buf, "{}", record.args())
    } else {
        writeln!(buf, "{}: {}", colored_level, record.args())
    }
}
