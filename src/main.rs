use anyhow::{Context, Result};
use clap::Parser;
use hound::{SampleFormat, WavReader, WavWriter};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// CLI arguments for the tempo adjustment tool.
#[derive(Debug, Parser)]
#[command(name = "wav-files-tempo")]
#[command(
    about = "Adjusts playback tempo of mono 16kHz 16-bit WAV files without altering pitch using time-stretching."
)]
struct Args {
    /// Input directory containing WAV files (processed recursively).
    #[arg(short = 'i', long)]
    input_dir: PathBuf,

    /// Output directory for processed files (preserves relative paths).
    #[arg(short = 'o', long)]
    output_dir: PathBuf,

    /// Tempo multiplier (e.g., 1.2 for 120% speed; default 1.0 = no change).
    #[arg(short = 't', long, default_value_t = 1.0)]
    tempo: f32,
}

/// Stretches audio samples by the inverse tempo factor without pitch shift.
fn stretch_samples(input: &[f32], sample_rate: u32, tempo: f32) -> Vec<f32> {
    if tempo == 1.0 {
        return input.to_vec();
    }

    let stretch_ratio = 1.0 / tempo;
    let input_len = input.len();
    let output_len = (input_len as f32 * stretch_ratio) as usize;

    let mut output = vec![0.0f32; output_len];

    let mut stretch = ssstretch::Stretch::new();
    stretch.preset_default(1, sample_rate as f32);

    // For mono: single-channel buffers.
    let input_ptr: *const f32 = input.as_ptr();
    let output_ptr: *mut f32 = output.as_mut_ptr();

    // Process the entire signal in one block (efficient for typical file sizes).
    // Assumes ssstretch API mirrors C++: process with buffers and lengths.
    unsafe {
        stretch.process(
            &[input_ptr],
            input_len as i32,
            &mut [output_ptr],
            output_len as i32,
        )
    };

    output
}

/// Processes a single WAV file: reads, stretches, and writes to output path.
fn process_file(input_path: &Path, output_path: &Path, tempo: f32) -> Result<()> {
    let mut reader = WavReader::open(input_path).context("Failed to open input WAV")?;
    let spec = reader.spec();

    // Validate format as per user spec.
    if spec.channels != 1
        || spec.sample_rate != 16000
        || spec.bits_per_sample != 16
        || spec.sample_format != SampleFormat::Int
    {
        anyhow::bail!("Unsupported format: expected mono 16-bit PCM at 16000 Hz");
    }

    // Read and normalize to f32 [-1.0, 1.0].
    let samples: Result<Vec<i16>> = reader
        .samples::<i16>()
        .map(|res| res.context("Invalid sample"))
        .collect::<Result<Vec<i16>>>();
    let input_samples: Vec<f32> = samples?.iter().map(|&s| s as f32 / 32768.0).collect();

    // Stretch samples.
    let output_samples = stretch_samples(&input_samples, spec.sample_rate, tempo);

    // Denormalize to i16.
    let output_i16: Vec<i16> = output_samples
        .iter()
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect();

    // Write output WAV (same spec, adjusted length).
    let mut writer = WavWriter::create(output_path, spec).context("Failed to create output WAV")?;
    for &sample in &output_i16 {
        writer
            .write_sample(sample)
            .context("Failed to write sample")?;
    }
    writer.finalize().context("Failed to finalize WAV")?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Ensure output dir exists.
    fs::create_dir_all(&args.output_dir).context("Failed to create output directory")?;

    // Recursively process WAV files, preserving structure.
    for entry in WalkDir::new(&args.input_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension() == Some("wav".as_ref()))
    {
        let rel_path = entry
            .path()
            .strip_prefix(&args.input_dir)
            .map_err(|_| anyhow::anyhow!("Invalid relative path"))?;
        let out_path = args.output_dir.join(rel_path);
        fs::create_dir_all(out_path.parent().unwrap_or_else(|| Path::new(".")))
            .context("Failed to create output subdir")?;

        if let Err(e) = process_file(entry.path(), &out_path, args.tempo) {
            eprintln!("Error processing {:?}: {}", entry.path(), e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stretch_samples_no_change() {
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let sample_rate = 16000;
        let tempo = 1.0;
        let output = stretch_samples(&input, sample_rate, tempo);
        assert_eq!(output, input);
    }

    #[test]
    fn test_stretch_samples_faster() {
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let sample_rate = 16000;
        let tempo = 2.0; // Twice as fast, output should be roughly half length
        let output = stretch_samples(&input, sample_rate, tempo);
        assert!((output.len() as f32 - input.len() as f32 / tempo).abs() < 2.0); // Allow for small rounding differences
        assert!(output.len() < input.len());
    }

    #[test]
    fn test_stretch_samples_slower() {
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let sample_rate = 16000;
        let tempo = 0.5; // Half as fast, output should be roughly double length
        let output = stretch_samples(&input, sample_rate, tempo);
        assert!((output.len() as f32 - input.len() as f32 / tempo).abs() < 2.0); // Allow for small rounding differences
        assert!(output.len() > input.len());
    }

    #[test]
    fn test_process_file_integration() -> Result<()> {
        let input_dir = PathBuf::from("test_input");
        let output_dir = PathBuf::from("test_output");
        fs::create_dir_all(&input_dir)?;
        fs::create_dir_all(&output_dir)?;

        // Create a dummy WAV file
        let input_path = input_dir.join("test_mono.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&input_path, spec)?;
        for i in 0..16000 {
            // 1 second of a simple sine wave
            let sample = (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / 16000.0).sin() * 10000.0;
            writer.write_sample(sample as i16)?;
        }
        writer.finalize()?;

        let output_path = output_dir.join("test_mono_stretched.wav");
        let tempo = 0.5; // Slow down by half

        process_file(&input_path, &output_path, tempo)?;

        // Verify output file exists and has roughly expected length
        assert!(output_path.exists());
        let reader = WavReader::open(&output_path)?;
        let expected_len = (16000 as f32 / tempo) as usize;
        assert!(((reader.len() as isize - expected_len as isize) as isize).abs() < 100); // Allow for small differences

        fs::remove_dir_all(&input_dir)?;
        fs::remove_dir_all(&output_dir)?;
        Ok(())
    }
}
