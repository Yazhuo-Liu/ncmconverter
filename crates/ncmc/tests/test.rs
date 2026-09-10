use id3::TagLike;
use ncm_core::{audio::Type as AudioType, decoder::Decoder};
use ncm_meta::Encoder;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

#[testing::fixture("tests/input/*.ncm")]
fn test_dump(input: PathBuf) {
    let reader = fs::File::open(&input).unwrap();

    let Decoder { key, comment, meta, image, mut audio } = Decoder::decode(reader).unwrap();

    if !meta.is_empty() {
        let meta = String::from_utf8_lossy(&meta);
        eprintln!("{meta}");
    };

    let key_path = input.with_extension("key");
    let expected_key = fs::read(key_path).unwrap();
    assert_eq!(key, expected_key);

    if !comment.is_empty() {
        let comment_path = input.with_extension("comment");
        let expected_comment = fs::read(comment_path).unwrap();
        assert_eq!(comment, expected_comment);
    }

    if !meta.is_empty() {
        let meta_path = input.with_extension("json");
        let expected_meta = fs::read(meta_path).unwrap();
        assert_eq!(meta, expected_meta);
    }

    if let Some(image) = image {
        let image_path = input.with_extension(image.ext());
        let expected_image = fs::read(image_path).unwrap();
        assert_eq!(image.data(), &expected_image);
    }

    let mut audio_data = Vec::new();
    audio.read_to_end(&mut audio_data).unwrap();
    let expected_hash = match input.file_name().unwrap().to_str().unwrap() {
        "1356233945.ncm" => "25f3345f7f2d905c5b3c263394e05c10b6993719d1e875abd591c025eab28d75",
        "1431343706.ncm" => "9670363357938d30fbc36bb1c8c9bc11c3b97770055f9e41c6118b1fef6598c6",
        "1434241304.ncm" => "a80844d337667802aa4de74bd5b2f7c7f6cc793b8c155da65b14529dbeb44749",
        "1463406864.ncm" => "39096b038723c7cd239f583a04aec2540c35c6f888fec58f5ec5174a37a78fde",
        "1481911318.ncm" => "4aff6de9ddcf6e2b2abd2804c53e126343e2b68523c4e49dd2e88eb6b49d308f",
        "1832399351.ncm" => "b96688dcb308d2de9c81a0b9d8f6fed3cd89dffb48c4e3f72e5156cbb12e963b",
        "31654862.ncm" => "344f9b37b1bd9fc3634a1701a8f55f075886371ea0081985530128057ac74b58",
        "509330088.ncm" => "60a05019269fbf8d8d8b45793be735ac024902bb9d3ca5200cff85102fbf480d",
        "583277.ncm" => "2dd838f56cf32360b8d7c66b04a340bec37324011f612f0ec2aa823e389c53ee",
        "807325.ncm" => "5e9c333d8449fc864353ccc7bae5f64d4a4248e7ff18848351d9fefd09523603",
        unexpected => panic!("unexpected fixture: {unexpected}"),
    };
    assert_eq!(format!("{:x}", Sha256::digest(audio_data)), expected_hash);
}

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);
const FIXTURE: &[u8] = include_bytes!("input/1356233945.ncm");

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("ncmc-cli-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn fixture(&self, relative: impl AsRef<Path>) -> PathBuf {
        let path = self.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, FIXTURE).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn audio_path(input: &Path) -> Option<PathBuf> {
    ["flac", "mp3", "m4a", "ogg", "audio"]
        .into_iter()
        .map(|extension| input.with_extension(extension))
        .find(|path| path.exists())
}

fn run_dump(
    args: impl IntoIterator<Item = PathBuf>,
    recursive: bool,
    skip_existing: bool,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ncmc"));
    command.arg("--dump");
    if recursive {
        command.arg("--recursive");
    }
    if skip_existing {
        command.arg("--skip-existing");
    }
    command.args(args);
    command.output().unwrap()
}

#[test]
fn cli_scans_directories_recursively_and_handles_unicode_paths() {
    let temp = TempDir::new();
    let root = temp.path().join("music library");
    let top = temp.fixture("music library/a.ncm");
    let unicode = temp.fixture("music library/中文 日本語 🎵.NCM");
    let nested = temp.fixture("music library/sub/b.ncm");
    fs::write(root.join("ignored.txt"), b"not an ncm file").unwrap();

    let shallow = run_dump([root.clone()], false, false);
    assert!(shallow.status.success(), "{}", String::from_utf8_lossy(&shallow.stderr));
    assert!(audio_path(&top).is_some());
    assert!(audio_path(&unicode).is_some());
    assert!(audio_path(&nested).is_none());
    assert!(String::from_utf8_lossy(&shallow.stderr).contains("total=2 succeeded=2"));

    let recursive = run_dump([root], true, false);
    assert!(recursive.status.success(), "{}", String::from_utf8_lossy(&recursive.stderr));
    assert!(audio_path(&nested).is_some());
    assert!(String::from_utf8_lossy(&recursive.stderr).contains("total=3 succeeded=3"));
}

#[test]
fn cli_accepts_mixed_file_and_directory_inputs() {
    let temp = TempDir::new();
    let direct = temp.fixture("direct cache file");
    let root = temp.path().join("directory");
    let from_directory = temp.fixture("directory/from-directory.ncm");

    let output = run_dump([direct.clone(), root], false, false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(audio_path(&direct).is_some());
    assert!(audio_path(&from_directory).is_some());
    assert!(String::from_utf8_lossy(&output.stderr).contains("total=2 succeeded=2"));
}

#[test]
fn cli_continues_after_a_bad_file_and_returns_failure() {
    let temp = TempDir::new();
    let root = temp.path().join("batch");
    let first = temp.fixture("batch/a-good.ncm");
    fs::write(root.join("b-broken.ncm"), b"not an ncm file").unwrap();
    let last = temp.fixture("batch/c-good.ncm");

    let output = run_dump([root], false, false);
    assert!(!output.status.success());
    assert!(audio_path(&first).is_some());
    assert!(audio_path(&last).is_some());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("b-broken.ncm"));
    assert!(stderr.contains("total=3 succeeded=2 skipped=0 failed=1"));
}

#[test]
fn cli_skip_existing_preserves_the_audio_output() {
    let temp = TempDir::new();
    let input = temp.fixture("song.ncm");
    let initial = run_dump([input.clone()], false, false);
    assert!(initial.status.success(), "{}", String::from_utf8_lossy(&initial.stderr));
    let output = audio_path(&input).unwrap();
    fs::write(&output, b"existing output").unwrap();

    let skipped = run_dump([input], false, true);
    assert!(skipped.status.success(), "{}", String::from_utf8_lossy(&skipped.stderr));
    assert_eq!(fs::read(output).unwrap(), b"existing output");
    assert!(
        String::from_utf8_lossy(&skipped.stderr).contains("total=1 succeeded=0 skipped=1 failed=0")
    );
}

#[test]
fn encoder_outputs_reopenable_mp3_and_flac_metadata() {
    let mp3 = fs::File::open("tests/input/1356233945.ncm").unwrap();
    let decoder = Decoder::decode(mp3).unwrap();
    assert!(matches!(decoder.audio_type(), AudioType::Mp3));
    let Encoder { data, .. } = Encoder::encode(decoder).unwrap();
    let tag = id3::Tag::read_from2(&mut Cursor::new(data)).unwrap();
    assert_eq!(tag.title(), Some("Eye Catch"));
    assert_eq!(tag.album(), Some("TVアニメ「私に天使が舞い降りた!」サウンドコレクション"));
    assert_eq!(tag.artist(), Some("伊賀拓郎"));
    assert!(tag.pictures().next().is_some());

    let flac = fs::File::open("tests/input/1431343706.ncm").unwrap();
    let decoder = Decoder::decode(flac).unwrap();
    assert!(matches!(decoder.audio_type(), AudioType::Flac));
    let Encoder { data, .. } = Encoder::encode(decoder).unwrap();
    let tag = metaflac::Tag::read_from(&mut Cursor::new(data)).unwrap();
    let comments = tag.vorbis_comments().unwrap();
    assert_eq!(comments.title(), Some(&vec!["アイキャッチ".into()]));
    assert_eq!(
        comments.album(),
        Some(&vec!["TVアニメ「ネコぱら」サウンドコレクションアルバム".into()])
    );
    assert_eq!(comments.artist(), Some(&vec!["立山秋航".into()]));
    assert!(tag.pictures().next().is_some());
}
