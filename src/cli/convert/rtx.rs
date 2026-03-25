use crate::opts::ConvertArgs;
use color_eyre::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::info;
use rayon::prelude::*;
use rgpre::import::rtx::{self, RtxEntry};
use serde_json::json;
use std::path::Path;

pub(crate) fn handle_rtx_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let rtx_file =
        rtx::parse_rtx_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    std::fs::create_dir_all(output_path)?;

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
                    let wav_path = output_path.join(format!("{tag_str}.wav"));

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
                        "wav_file": format!("{tag_str}.wav"),
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
