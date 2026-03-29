use color_eyre::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::info;
use rayon::prelude::*;
use rgpre::import::sfx;
use serde_json::json;
use std::path::Path;

pub(crate) fn handle_sfx_convert(file: &Path, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(file)?;
    let sfx_file =
        sfx::parse_sfx_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    std::fs::create_dir_all(output_path)?;

    let effect_metadata: Vec<serde_json::Value> = sfx_file
        .effects
        .par_iter()
        .enumerate()
        .map(|(i, effect)| -> Result<serde_json::Value> {
            let wav_filename = format!("{i:03}.wav");
            let wav_path = output_path.join(&wav_filename);

            let spec = WavSpec {
                channels: effect.audio_type.channels(),
                sample_rate: effect.sample_rate,
                bits_per_sample: effect.audio_type.bits_per_sample(),
                sample_format: SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&wav_path, spec)?;

            if effect.audio_type.bits_per_sample() == 8 {
                for &sample in &effect.pcm_data {
                    writer.write_sample((sample as i16 - 128) as i8)?;
                }
            } else {
                debug_assert!(
                    effect.pcm_data.len().is_multiple_of(2),
                    "16-bit PCM data has odd byte count: {}",
                    effect.pcm_data.len()
                );
                for chunk in effect.pcm_data.chunks_exact(2) {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    writer.write_sample(sample)?;
                }
            }

            writer.finalize()?;

            info!(
                "  [{i:03}] {:?} {}Hz {:.3}s -> {}",
                effect.audio_type,
                effect.sample_rate,
                effect.duration_secs(),
                wav_path.display(),
            );

            Ok(json!({
                "index": i,
                "wav_file": wav_filename,
                "audio_type": format!("{:?}", effect.audio_type),
                "sample_rate": effect.sample_rate,
                "duration_secs": effect.duration_secs(),
                "loop": effect.loop_flag != 0,
                "loop_offset": effect.loop_offset,
                "loop_end": effect.loop_end,
                "pcm_bytes": effect.pcm_data.len(),
            }))
        })
        .collect::<Result<Vec<_>>>()?;

    let source_name = file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let index = json!({
        "source": source_name,
        "description": sfx_file.description,
        "effect_count": effect_metadata.len(),
        "effects": effect_metadata,
    });

    let index_path = output_path.join("index.json");
    std::fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;
    info!("Metadata index written to: {}", index_path.display());

    info!(
        "Extracted {} effects to {}",
        sfx_file.effects.len(),
        output_path.display()
    );

    Ok(())
}
