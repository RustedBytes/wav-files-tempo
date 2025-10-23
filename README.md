# wav-files-tempo

A command-line tool to adjust the playback tempo (speed) of mono 16kHz 16-bit PCM WAV audio files without altering the pitch. It uses time-stretching algorithms (phase-vocoder based, similar to WSOLA) to achieve natural-sounding speed changes, ideal for speech or music processing.

## Features

- **Recursive Processing**: Scans input directories (including subfolders) for `.wav` files.
- **Pitch-Preserving Tempo Adjustment**: Change speed by a multiplier (e.g., 1.2x faster) while keeping original pitch intact.
- **Format Validation**: Ensures input files match the specified format (mono, 16-bit PCM, 16000 Hz).
- **Output Preservation**: Maintains directory structure in the output folder.
- **Efficient & Safe**: Built in Rust for memory safety and performance; processes files in-memory for typical sizes.

## Installation

Requires Rust toolchain (stable) installed via [rustup](https://rustup.rs/).

## Usage

```bash
wav-files-tempo [OPTIONS]
```

### Required Arguments

- `-i, --input-dir <INPUT_DIR>`: Input directory containing WAV files (processed recursively).
- `-o, --output-dir <OUTPUT_DIR>`: Output directory for processed files (structure preserved).

### Optional Arguments

- `-t, --tempo <TEMPO>`: Tempo multiplier (default: `1.0`). Values >1.0 speed up; <1.0 slow down. E.g., `1.5` for 150% speed.

Run `wav-files-tempo --help` for full details.

## Examples

### Basic Usage: Speed Up Files by 20%

```bash
wav-files-tempo -i ./input_audio -o ./output_audio -t 1.2
```

This processes all `.wav` files in `./input_audio` (and subdirs), outputs tempo-adjusted versions to `./output_audio`, preserving paths.

### No Change (Identity)

```bash
wav-files-tempo -i ./input -o ./output -t 1.0
```

Files are copied as-is (useful for batch validation).

### Slow Down to 80% Speed

```bash
wav-files-tempo -i ./speeches -o ./slowed -t 0.8
```

## Building from Source

Clone the repo:

```bash
git clone https://github.com/RustedBytes/wav-files-tempo.git
cd wav-files-tempo
```

Build and install:

```bash
cargo build --release
cargo install --path .
```

Or run directly:

```bash
cargo run -- -i ./test -o ./out -t 1.5
```

Dependencies are minimal: `clap`, `hound`, `ssstretch`, `walkdir`, `anyhow`.

## Testing

Run the test suite:

```bash
cargo test
```

Includes unit tests for stretching logic, file I/O, format validation, and edge cases (e.g., identity tempo, constant signals).

## Performance Notes

- Optimized for files <10s (in-memory processing).
- For longer files, artifacts may occur at extreme tempos (>2x or <0.5x); test with your data.
- Processes mono only; extend for stereo if needed (future feature).

## License

MIT License - see [LICENSE](LICENSE) file.
