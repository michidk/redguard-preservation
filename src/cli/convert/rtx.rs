use crate::opts::ConvertArgs;
use color_eyre::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::info;
use rayon::prelude::*;
use rgpre::import::rtx::{self, RtxEntry};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

const MAX_FILENAME_LEN: usize = 80;

fn sanitize_filename(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut prev_underscore = false;

    for c in name.chars() {
        match c {
            'A'..='Z' => {
                result.push(c.to_ascii_lowercase());
                prev_underscore = false;
            }
            'a'..='z' | '0'..='9' => {
                result.push(c);
                prev_underscore = false;
            }
            ' ' | '_' | '-' => {
                if !prev_underscore && !result.is_empty() {
                    result.push('_');
                    prev_underscore = true;
                }
            }
            _ => {}
        }
    }

    while result.ends_with('_') {
        result.pop();
    }

    if result.len() > MAX_FILENAME_LEN {
        if let Some(pos) = result[..MAX_FILENAME_LEN].rfind('_') {
            result.truncate(pos);
        } else {
            result.truncate(MAX_FILENAME_LEN);
        }
    }

    result
}

fn build_resolved_filenames(entries: &[RtxEntry]) -> Vec<String> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    entries
        .iter()
        .map(|entry| {
            let raw = match entry {
                RtxEntry::Audio { label, .. } => label.as_str(),
                RtxEntry::Text { text, .. } => text.as_str(),
            };

            let base = sanitize_filename(raw);
            if base.is_empty() {
                return entry.tag_str();
            }

            let count = counts.entry(base.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base}_{count}")
            }
        })
        .collect()
}

pub(crate) fn handle_rtx_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let rtx_file =
        rtx::parse_rtx_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    std::fs::create_dir_all(output_path)?;

    let resolved = if args.resolve_names {
        Some(build_resolved_filenames(&rtx_file.entries))
    } else {
        None
    };

    let metadata_entries: Vec<serde_json::Value> = rtx_file
        .entries
        .par_iter()
        .enumerate()
        .map(|(i, entry)| {
            let tag_str = entry.tag_str();

            match entry {
                RtxEntry::Text { text, .. } => Ok(json!({
                    "index": i,
                    "tag": tag_str,
                    "type": "text",
                    "text": text,
                })),
                RtxEntry::Audio {
                    label,
                    header,
                    pcm_data,
                    ..
                } => {
                    let wav_stem = resolved
                        .as_ref()
                        .map_or_else(|| tag_str.clone(), |names| names[i].clone());
                    let wav_filename = format!("{wav_stem}.wav");
                    let wav_path = output_path.join(&wav_filename);

                    let spec = WavSpec {
                        channels: header.audio_type.channels(),
                        sample_rate: header.sample_rate,
                        bits_per_sample: header.audio_type.bits_per_sample(),
                        sample_format: SampleFormat::Int,
                    };

                    let mut writer = WavWriter::create(&wav_path, spec)?;

                    if header.audio_type.bits_per_sample() == 8 {
                        for &sample in pcm_data {
                            writer.write_sample(sample.cast_signed())?;
                        }
                    } else {
                        debug_assert!(
                            pcm_data.len().is_multiple_of(2),
                            "16-bit PCM data has odd byte count: {}",
                            pcm_data.len()
                        );
                        for chunk in pcm_data.chunks_exact(2) {
                            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                            writer.write_sample(sample)?;
                        }
                    }

                    writer.finalize()?;

                    info!(
                        "  [{i:04}] '{}' {:?} {}Hz {:.3}s -> {}",
                        tag_str,
                        header.audio_type,
                        header.sample_rate,
                        header.duration_secs(),
                        wav_path.display(),
                    );

                    Ok(json!({
                        "index": i,
                        "tag": tag_str,
                        "type": "audio",
                        "label": label,
                        "audio_type": format!("{:?}", header.audio_type),
                        "sample_rate": header.sample_rate,
                        "duration_secs": header.duration_secs(),
                        "pcm_bytes": pcm_data.len(),
                        "wav_file": wav_filename,
                    }))
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    let audio_written = metadata_entries
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("audio"))
        .count();

    let index_path = output_path.join("index.json");
    let index_json = json!({
        "entry_count": rtx_file.entries.len(),
        "audio_count": rtx_file.audio_count(),
        "text_count": rtx_file.text_count(),
        "entries": metadata_entries,
    });
    let json_text = serde_json::to_string_pretty(&index_json)?;
    std::fs::write(&index_path, json_text)?;
    info!("Metadata index written to: {}", index_path.display());

    info!(
        "Extracted {} audio clips + {} text entries to {}",
        audio_written,
        rtx_file.text_count(),
        output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_basic_name() {
        assert_eq!(sanitize_filename("SABRE"), "sabre");
    }

    #[test]
    fn sanitize_multi_word() {
        assert_eq!(sanitize_filename("SOUL SWORD"), "soul_sword");
    }

    #[test]
    fn sanitize_apostrophe() {
        assert_eq!(sanitize_filename("SPIDER'S MILK"), "spiders_milk");
    }

    #[test]
    fn sanitize_sentence() {
        assert_eq!(
            sanitize_filename("GET BACK IN YOUR JAR, YOU FILTHY LITTLE THING."),
            "get_back_in_your_jar_you_filthy_little_thing"
        );
    }

    #[test]
    fn sanitize_question_mark() {
        assert_eq!(
            sanitize_filename("WHICH DOOR IS CORRECT?"),
            "which_door_is_correct"
        );
    }

    #[test]
    fn sanitize_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    #[test]
    fn sanitize_truncates_long_name() {
        let long = "A ".repeat(100);
        let result = sanitize_filename(&long);
        assert!(result.len() <= MAX_FILENAME_LEN);
        assert!(!result.ends_with('_'));
    }

    #[test]
    fn dedup_identical_names() {
        use rgpre::import::rtx::RtxEntry;

        let entries = vec![
            RtxEntry::Text {
                tag: *b"xr01",
                text: "RUNE".to_string(),
            },
            RtxEntry::Text {
                tag: *b"xr02",
                text: "RUNE".to_string(),
            },
            RtxEntry::Text {
                tag: *b"xr03",
                text: "RUNE".to_string(),
            },
        ];
        let names = build_resolved_filenames(&entries);
        assert_eq!(names[0], "rune");
        assert_eq!(names[1], "rune_2");
        assert_eq!(names[2], "rune_3");
    }
}
