# ncmc &middot; [![Test](https://github.com/magic-akari/ncmc/actions/workflows/test.yml/badge.svg)](https://github.com/magic-akari/ncmc/actions/workflows/test.yml) [![Crates.io](https://img.shields.io/crates/v/ncmc.svg?label=ncmc)](https://crates.io/crates/ncmc)

## Install

### Option 1: Download from GitHub Release

For users who prefer a pre-built binary, you can download the latest release from [![GitHub Release](https://img.shields.io/badge/build-Release-brightgreen?style=flat&logo=github&label=GitHub)](https://github.com/magic-akari/ncmc/releases/latest)

### Option 2: Install with Cargo

If you have Rust installed, you can install ncmc with cargo:

```bash
cargo install ncmc
```

Additionally, [cargo binstall](https://github.com/cargo-bins/cargo-binstall) is supported.
It fetch the pre-built binary from GitHub Release and fallback to build from source if not available.

```bash
cargo binstall ncmc
```

If you don’t have cargo, install it with
https://rustup.rs

## Usage

```bash
# convert one file
ncmc path/to/your/file.ncm

# scan only the top level of a directory (.ncm and .NCM files)
ncmc path/to/music-directory

# recursively scan one or more directories; direct file inputs can be mixed in
ncmc --recursive path/to/music-directory another/file.ncm

# do not overwrite audio output files that already exist
ncmc --recursive --skip-existing path/to/music-directory

# dump decrypted key, metadata, cover, and audio data
ncmc --dump path/to/your/file.ncm
```

Directory scans only select NCM files, use ASCII case-insensitive extension matching, and do not
follow symlinks found inside a scanned directory. Direct file inputs are not extension-filtered.
Conversions run serially. A failed file is reported while the remaining files continue; each run
ends with a `total`, `succeeded`, `skipped`, and `failed` summary. Any failed discovery or
conversion makes the command exit with a non-zero status.

---

Thanks: [anonymous5l / ncmdump](https://github.com/anonymous5l/ncmdump)
