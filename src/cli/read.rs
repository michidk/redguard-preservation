//! Read command handler

use super::utils::resolve_filetype;
use crate::opts::ReadArgs;
use color_eyre::Result;
use log::info;
use redguard_preservation::import::{
    bsi, cht, fnt, model3d, palette::Palette, pvo, rgm, rob, rtx, sfx, wld,
};
use std::collections::BTreeMap;

pub(crate) fn handle_read_command(args: ReadArgs) -> Result<()> {
    let file_path = &args.file;
    let filetype = resolve_filetype(file_path, args.filetype)?;

    info!("Reading file: {}", file_path.display());
    info!("File type: {filetype:?}");

    let file_content = std::fs::read(file_path)?;

    match filetype {
        redguard_preservation::import::FileType::Cht => {
            let file =
                cht::parse_cht_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            info!("Successfully parsed CHT file");

            for entry in file.named_cheats() {
                let name = entry.name.unwrap_or("?");
                let state = if entry.is_on() { "ON" } else { "off" };
                if entry.value > 1 {
                    info!(
                        "  [{:2}] {:<14} {} (value={})",
                        entry.index, name, state, entry.value
                    );
                } else {
                    info!("  [{:2}] {:<14} {}", entry.index, name, state);
                }
            }

            let unexpected = file.nonzero_unnamed();
            if !unexpected.is_empty() {
                info!("Nonzero unnamed slots:");
                for entry in unexpected {
                    info!("  [{:2}] value={}", entry.index, entry.value);
                }
            }
        }
        redguard_preservation::import::FileType::Rob => {
            let (rob_file, models) = rob::parse_rob_with_models(&file_content)?;
            info!("Successfully parsed ROB file!");
            info!("Header: {:?}", rob_file.header);
            info!("Number of segments: {}", rob_file.segments.len());

            // Count different types of segments
            let embedded_count = rob_file
                .segments
                .iter()
                .filter(|s| s.has_embedded_3d_data())
                .count();
            let external_count = rob_file
                .segments
                .iter()
                .filter(|s| s.points_to_external_file())
                .count();
            let other_count = rob_file.segments.len() - embedded_count - external_count;

            info!("Number of embedded 3D models: {embedded_count}");
            info!("Number of referenced 3D models: {external_count}");
            info!("Number of other segments: {other_count}");

            // Print segment information
            for (i, segment) in rob_file.segments.iter().enumerate() {
                let name = segment.name();

                if segment.points_to_external_file() {
                    info!("Segment {i}: '{name}' points to external 3DC file");
                } else if segment.has_embedded_3d_data() {
                    info!(
                        "Segment {}: '{}' embeds 3D data (size: {})",
                        i, name, segment.data_size
                    );
                } else {
                    info!(
                        "Segment {}: '{}' contains other data (size: {})",
                        i, name, segment.data_size
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
                info!("  Vertex normals: {}", model.vertex_normals.len());

                if let Some((min, max)) = model.bounding_box() {
                    info!("  Bounding box:");
                    info!("    Min: ({:.2}, {:.2}, {:.2})", min.x, min.y, min.z);
                    info!("    Max: ({:.2}, {:.2}, {:.2})", max.x, max.y, max.z);
                }
            }
        }
        redguard_preservation::import::FileType::Model3d
        | redguard_preservation::import::FileType::Model3dc => {
            let model = model3d::parse_3d_file(&file_content)?;
            info!("Successfully parsed 3D model file!");
            info!("Version: {}", model.header.version_string());
            info!("Vertices: {}", model.header.num_vertices);
            info!("Faces: {}", model.header.num_faces);
            info!("Total face vertices: {}", model.total_face_vertices());
            info!("Vertex normals: {}", model.vertex_normals.len());

            if let Some((min, max)) = model.bounding_box() {
                info!("Bounding box:");
                info!("  Min: ({:.2}, {:.2}, {:.2})", min.x, min.y, min.z);
                info!("  Max: ({:.2}, {:.2}, {:.2})", max.x, max.y, max.z);
            }
        }
        redguard_preservation::import::FileType::Fnt => {
            let font = fnt::parse_fnt(&file_content)?;
            info!("Successfully parsed FNT file!");
            info!("Chunk order: {}", font.chunk_order.join(" -> "));
            info!(
                "Header description: {}",
                if font.header.description_text.is_empty() {
                    "<empty>"
                } else {
                    &font.header.description_text
                }
            );
            info!("Line height: {}", font.header.line_height);
            info!("Max width: {}", font.header.max_width);
            info!("Character start: {}", font.header.character_start);
            info!("Character count: {}", font.header.character_count);
            info!(
                "Palette tag: {}",
                String::from_utf8_lossy(&font.palette.tag)
            );
            info!("Palette colors: {}", font.palette.colors.len());

            let enabled = font.glyphs.iter().filter(|g| g.enabled != 0).count();
            info!("Enabled glyphs: {} / {}", enabled, font.glyphs.len());
            info!("Has RDAT: {}", font.rdat.is_some());

            if !font.trailing_padding.is_empty() {
                info!(
                    "Trailing zero padding bytes after END: {}",
                    font.trailing_padding.len()
                );
            }
        }
        redguard_preservation::import::FileType::Pvo => {
            let file = pvo::parse_pvo_file(&file_content)?;
            info!("Successfully parsed PVO file");
            info!(
                "Header: depth={}, total_nodes={}, leaf_nodes={}, interior_nodes={}",
                file.header.depth,
                file.header.total_nodes,
                file.header.leaf_nodes,
                file.header.interior_nodes()
            );
            info!(
                "Center: ({}, {}, {}), cell_size={}",
                file.header.center_x,
                file.header.center_y,
                file.header.center_z,
                file.header.cell_size
            );
            info!(
                "OCTR: {} nodes ({} interior, {} leaf-only)",
                file.octr_nodes.len(),
                file.count_interior_nodes(),
                file.count_leaf_nodes()
            );
            let total_plst_entries: usize = file.plst_leaves.iter().map(|l| l.entries.len()).sum();
            info!(
                "PLST: {} leaves, {} total entries",
                file.plst_leaves.len(),
                total_plst_entries
            );
            info!(
                "MLST: {} polygon indices (header says {})",
                file.mlst_indices.len(),
                file.header.mlst_polygon_count
            );
        }
        redguard_preservation::import::FileType::Wld => {
            let file = wld::parse_wld_file(&file_content)?;
            let offsets = file.header.section_offsets();
            info!("Successfully parsed WLD file");
            info!(
                "Header fields: field_00={}, section_cols={}, section_rows={}, section_header_size={}",
                file.header.fields[0],
                file.header.fields[1],
                file.header.fields[2],
                file.header.fields[6]
            );
            info!(
                "Section offsets: [{}, {}, {}, {}]",
                offsets[0], offsets[1], offsets[2], offsets[3]
            );

            for (idx, section) in file.sections.iter().enumerate() {
                let nonzero_map1 = section.maps[0].iter().filter(|v| **v != 0).count();
                let nonzero_map2 = section.maps[1].iter().filter(|v| **v != 0).count();
                let nonzero_map3 = section.maps[2].iter().filter(|v| **v != 0).count();
                let nonzero_map4 = section.maps[3].iter().filter(|v| **v != 0).count();
                info!(
                    "Section[{idx}] map non-zero counts: m1={nonzero_map1}, m2={nonzero_map2}, m3={nonzero_map3}, m4={nonzero_map4}"
                );
            }

            let footer_hex = file
                .footer
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            info!("Footer bytes: {footer_hex}");
        }
        redguard_preservation::import::FileType::Rgm => {
            let file = rgm::parse_rgm_file(&file_content)?;
            info!("Successfully parsed RGM file");
            info!("Section count: {}", file.sections.len());

            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for section in &file.sections {
                let name = section.header().name();
                *counts.entry(name).or_insert(0) += 1;
            }

            for (name, count) in counts {
                info!("Section {name}: {count}");
            }
        }
        redguard_preservation::import::FileType::Bsi => {
            let file = bsi::parse_bsi_file(&file_content)?;
            info!("Successfully parsed BSI file");
            info!("Image count: {}", file.images.len());

            if let Some(first) = file.images.first() {
                info!(
                    "First image: name='{}' index={} size={}x{} animated={} frames={}",
                    first.name,
                    first.image_index,
                    first.width,
                    first.height,
                    first.is_animated,
                    first.frame_count
                );
            }
        }
        redguard_preservation::import::FileType::Col => {
            let palette = Palette::parse(&file_content)?;
            info!("Palette data loaded (minimum size/layout validation)");
            info!("Color count: {}", palette.colors.len());
            let first = palette.colors[0];
            let mid = palette.colors[128];
            let last = palette.colors[255];
            info!(
                "Sample colors: [0]=({},{},{}), [128]=({},{},{}), [255]=({},{},{})",
                first[0], first[1], first[2], mid[0], mid[1], mid[2], last[0], last[1], last[2]
            );
        }
        redguard_preservation::import::FileType::Sfx => {
            let file = sfx::parse_sfx_file(&file_content)?;
            info!("Successfully parsed SFX file");
            info!("Description: {}", file.description);
            info!("Effect count: {}", file.effects.len());

            for (i, effect) in file.effects.iter().enumerate() {
                info!(
                    "  [{i:03}] {:?} {}Hz {:.3}s loop={} ({} bytes)",
                    effect.audio_type,
                    effect.sample_rate,
                    effect.duration_secs(),
                    effect.loop_flag,
                    effect.pcm_data.len(),
                );
            }
        }
        redguard_preservation::import::FileType::Rtx => {
            let file =
                rtx::parse_rtx_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
            info!("Successfully parsed RTX file");
            info!("Index count: {}", file.index_count);
            info!("Total entries: {}", file.entries.len());
            info!(
                "Audio entries: {}, Text entries: {}",
                file.audio_count(),
                file.text_count()
            );

            for (i, entry) in file.entries.iter().enumerate() {
                match entry {
                    rtx::RtxEntry::Text { text, .. } => {
                        let preview = if text.len() > 60 {
                            format!("{}...", &text[..60])
                        } else {
                            text.clone()
                        };
                        info!("  [{i:04}] '{}' TEXT \"{}\"", entry.tag_str(), preview);
                    }
                    rtx::RtxEntry::Audio {
                        label,
                        header,
                        pcm_data,
                        ..
                    } => {
                        let preview = if label.len() > 40 {
                            format!("{}...", &label[..40])
                        } else {
                            label.clone()
                        };
                        info!(
                            "  [{i:04}] '{}' AUDIO {:?} {}Hz {:.3}s ({} bytes) \"{}\"",
                            entry.tag_str(),
                            header.audio_type,
                            header.sample_rate,
                            header.duration_secs(),
                            pcm_data.len(),
                            preview,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
