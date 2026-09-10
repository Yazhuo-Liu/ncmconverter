use anyhow::{Context, Result};
use ncm_core::decoder::Decoder;
use ncm_meta::Encoder;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExistingPolicy {
    Overwrite,
    Skip,
}

pub enum Outcome {
    Written { output: PathBuf, meta: String },
    Skipped { output: PathBuf },
}

pub fn auto_one(path: &Path, existing_policy: ExistingPolicy) -> Result<Outcome> {
    let reader = fs::File::open(path).with_context(|| format!("input {}", path.display()))?;
    let decoder = Decoder::decode(reader).with_context(|| format!("decode {}", path.display()))?;
    let output = path.with_extension(decoder.ext());

    if existing_policy == ExistingPolicy::Skip && output.exists() {
        return Ok(Outcome::Skipped { output });
    }

    let Encoder { data, meta } =
        Encoder::encode(decoder).with_context(|| format!("encode {}", path.display()))?;
    fs::write(&output, data).with_context(|| format!("write {}", output.display()))?;

    Ok(Outcome::Written { output, meta })
}

pub fn dump_one(path: &Path, existing_policy: ExistingPolicy) -> Result<Outcome> {
    let reader = fs::File::open(path).with_context(|| format!("input {}", path.display()))?;
    let Decoder { key, comment, meta, image, mut audio } =
        Decoder::decode(reader).with_context(|| format!("decode {}", path.display()))?;
    let audio_path = path.with_extension(audio.ext());

    if existing_policy == ExistingPolicy::Skip && audio_path.exists() {
        return Ok(Outcome::Skipped { output: audio_path });
    }

    let meta_message = if meta.is_empty() {
        "meta not found".into()
    } else {
        String::from_utf8_lossy(&meta).into_owned()
    };

    fs::write(path.with_extension("key"), key)
        .with_context(|| format!("write key for {}", path.display()))?;

    if !comment.is_empty() {
        fs::write(path.with_extension("comment"), comment)
            .with_context(|| format!("write comment for {}", path.display()))?;
    }

    if !meta.is_empty() {
        fs::write(path.with_extension("json"), meta)
            .with_context(|| format!("write metadata for {}", path.display()))?;
    }

    if let Some(image) = image {
        fs::write(path.with_extension(image.ext()), image.data())
            .with_context(|| format!("write cover for {}", path.display()))?;
    }

    let mut file = fs::File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&audio_path)
        .with_context(|| format!("write audio {}", audio_path.display()))?;
    io::copy(&mut audio, &mut file)
        .with_context(|| format!("write audio {}", audio_path.display()))?;

    Ok(Outcome::Written { output: audio_path, meta: meta_message })
}

#[cfg(test)]
mod tests {
    use super::{dump_one, ExistingPolicy, Outcome};
    use std::{
        env, fs,
        io::Write,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);
    const FIXTURE: &[u8] = include_bytes!("../tests/input/1356233945.ncm");

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!("ncmc-process-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn input(&self) -> PathBuf {
            let path = self.0.join("input.ncm");
            fs::write(&path, FIXTURE).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn written_output(outcome: Outcome) -> PathBuf {
        match outcome {
            Outcome::Written { output, .. } => output,
            Outcome::Skipped { .. } => panic!("fixture unexpectedly skipped"),
        }
    }

    #[test]
    fn dump_truncates_a_preexisting_audio_file() {
        let temp = TempDir::new();
        let input = temp.input();
        let output = written_output(dump_one(&input, ExistingPolicy::Overwrite).unwrap());
        let expected_len = fs::metadata(&output).unwrap().len();

        let mut file = fs::File::options().append(true).open(&output).unwrap();
        file.write_all(b"stale trailing bytes").unwrap();
        assert!(fs::metadata(&output).unwrap().len() > expected_len);

        let output_after_retry =
            written_output(dump_one(&input, ExistingPolicy::Overwrite).unwrap());
        assert_eq!(output_after_retry, output);
        assert_eq!(fs::metadata(&output).unwrap().len(), expected_len);
    }

    #[test]
    fn skip_existing_preserves_the_existing_audio_file() {
        let temp = TempDir::new();
        let input = temp.input();
        let output = written_output(dump_one(&input, ExistingPolicy::Overwrite).unwrap());
        fs::write(&output, b"do not overwrite").unwrap();

        match dump_one(&input, ExistingPolicy::Skip).unwrap() {
            Outcome::Skipped { output: skipped } => assert_eq!(skipped, output),
            Outcome::Written { .. } => panic!("existing audio was overwritten"),
        }
        assert_eq!(fs::read(output).unwrap(), b"do not overwrite");
    }
}
