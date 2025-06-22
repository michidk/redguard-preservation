//! Command-line interface for Redguard Preservation
//!
//! This binary provides a command-line interface for parsing ROB and 3D model files.

use clap::Parser;
use color_eyre::Result;
use log::{error, info, trace};
use std::io::Write;
mod opts;
use opts::{Commands, FileType, Opts};
use redguard_preservation::{
    convert_models_to_gltf, parse_rob_with_models, parser::parse_3d_file, to_glb,
};

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
                        Ok((_rob_file, models)) => {
                            info!("Successfully parsed ROB file!");
                            info!("Header: {:?}", _rob_file.header);
                            info!("Number of segments: {}", _rob_file.segments.len());

                            // Count different types of segments
                            let embedded_count = _rob_file
                                .segments
                                .iter()
                                .filter(|s| s.has_embedded_3d_data())
                                .count();
                            let external_count = _rob_file
                                .segments
                                .iter()
                                .filter(|s| s.points_to_external_file())
                                .count();
                            let other_count =
                                _rob_file.segments.len() - embedded_count - external_count;

                            info!("Number of embedded 3D models: {}", embedded_count);
                            info!("Number of referenced 3D models: {}", external_count);
                            info!("Number of other segments: {}", other_count);

                            // Print segment information
                            for (i, segment) in _rob_file.segments.iter().enumerate() {
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
        Commands::Convert { args } => {
            let file_path = &args.file;
            let filetype = args.filetype.unwrap_or_else(|| {
                FileType::from_extension(file_path).unwrap_or_else(|| {
                    error!("Could not infer file type from extension. Please specify --filetype");
                    std::process::exit(1);
                })
            });

            // Determine output filename
            let output_path = args.output.unwrap_or_else(|| {
                let mut path = file_path.clone();
                path.set_extension("glb");
                path
            });

            info!(
                "Converting file: {} to {}",
                file_path.display(),
                output_path.display()
            );

            let file_content = std::fs::read(file_path).map_err(|e| {
                color_eyre::eyre::eyre!("Failed to read file '{}': {}", file_path.display(), e)
            })?;

            let models = match filetype {
                FileType::Rob => match parse_rob_with_models(&file_content) {
                    Ok((_rob_file, models)) => {
                        info!(
                            "Successfully parsed ROB file, found {} models",
                            models.len()
                        );
                        models
                    }
                    Err(e) => {
                        error!("Failed to parse ROB file: {}", e);
                        std::process::exit(1);
                    }
                },
                FileType::Model3D => match parse_3d_file(&file_content) {
                    Ok(model) => {
                        info!("Successfully parsed 3D model file");
                        vec![model]
                    }
                    Err(e) => {
                        error!("Failed to parse 3D model file: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            if models.is_empty() {
                error!("No 3D models found to convert.");
                std::process::exit(1);
            }

            // Convert all models into a single GLB file
            match convert_models_to_gltf(&models) {
                Ok((root, buffer)) => match to_glb(&root, &buffer) {
                    Ok(glb_data) => {
                        std::fs::write(&output_path, glb_data).map_err(|e| {
                            color_eyre::eyre::eyre!(
                                "Failed to write GLB file '{}': {}",
                                output_path.display(),
                                e
                            )
                        })?;
                        info!("Successfully converted to: {}", output_path.display());
                    }
                    Err(e) => {
                        error!("Failed to serialize to GLB: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    error!("Failed to convert models to GLTF: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Dump {
            args,
            model,
            max_faces,
        } => {
            let file_path = &args.file;
            let filetype = args.filetype.unwrap_or_else(|| {
                FileType::from_extension(file_path).unwrap_or_else(|| {
                    error!("Could not infer file type from extension. Please specify --filetype");
                    std::process::exit(1);
                })
            });

            let file_content = std::fs::read(file_path).map_err(|e| {
                color_eyre::eyre::eyre!("Failed to read file '{}': {}", file_path.display(), e)
            })?;

            let models = match filetype {
                FileType::Rob => match parse_rob_with_models(&file_content) {
                    Ok((_rob_file, models)) => models,
                    Err(e) => {
                        error!("Failed to parse ROB file: {}", e);
                        std::process::exit(1);
                    }
                },
                FileType::Model3D => match parse_3d_file(&file_content) {
                    Ok(model) => vec![model],
                    Err(e) => {
                        error!("Failed to parse 3D model file: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            if models.is_empty() {
                error!("No 3D models found.");
                std::process::exit(1);
            }

            let targets: Vec<(usize, &redguard_preservation::model3d::Model3DFile)> =
                if let Some(idx) = model {
                    if idx >= models.len() {
                        error!("Model index {} out of range ({} models)", idx, models.len());
                        std::process::exit(1);
                    }
                    vec![(idx, &models[idx])]
                } else {
                    models.iter().enumerate().collect()
                };

            for (i, m) in targets {
                println!("\n--- Model {} ---", i);
                println!(
                    "Version: {} (raw: {:?})",
                    m.header.version_string(),
                    m.header.version
                );
                println!("Vertices: {}", m.vertex_coords.len());
                println!("Faces: {}", m.face_data.len());

                println!("First 10 vertices (scaled):");
                for (vi, v) in m.vertex_coords.iter().take(10).enumerate() {
                    println!("  {:>3}: ({:.3}, {:.3}, {:.3})", vi, v.x, v.y, v.z);
                }

                println!("First {} faces (as vertex indices):", max_faces);
                for (fi, face) in m.face_data.iter().take(max_faces).enumerate() {
                    let indices: Vec<u32> = face
                        .face_vertices
                        .iter()
                        .map(|fv| fv.vertex_index)
                        .collect();
                    println!("  Face {:>3}: {:?}", fi, indices);
                }
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
