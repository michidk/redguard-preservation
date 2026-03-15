use crate::opts::ConvertArgs;
use color_eyre::Result;
use hound::{SampleFormat, WavSpec, WavWriter};
use log::info;
use redguard_preservation::import::sfx;
use std::path::Path;

pub(super) fn handle_sfx_convert(args: &ConvertArgs, output_path: &Path) -> Result<()> {
    let file_content = std::fs::read(&args.file)?;
    let sfx_file =
        sfx::parse_sfx_file(&file_content).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    std::fs::create_dir_all(output_path)?;

    for (i, effect) in sfx_file.effects.iter().enumerate() {
        let wav_path = output_path.join(format!("{i:03}.wav"));

        let spec = WavSpec {
            channels: effect.audio_type.channels(),
            sample_rate: effect.sample_rate,
            bits_per_sample: effect.audio_type.bits_per_sample(),
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&wav_path, spec)?;

        if effect.audio_type.bits_per_sample() == 8 {
            for &sample in &effect.pcm_data {
                writer.write_sample(sample as i8)?;
            }
        } else {
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
    }

    info!(
        "Extracted {} effects to {}",
        sfx_file.effects.len(),
        output_path.display()
    );

    Ok(())
}
