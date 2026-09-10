use anyhow::Context;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Discovery {
    pub files: Vec<PathBuf>,
    pub errors: Vec<DiscoveryError>,
}

pub struct DiscoveryError {
    pub path: PathBuf,
    pub error: anyhow::Error,
}

impl Discovery {
    pub fn discover(inputs: &[PathBuf], recursive: bool) -> Self {
        let mut discovery = Self { files: Vec::new(), errors: Vec::new() };

        for input in inputs {
            match fs::metadata(input) {
                Ok(metadata) if metadata.is_file() => discovery.files.push(input.clone()),
                Ok(metadata) if metadata.is_dir() => discovery.visit_directory(input, recursive),
                Ok(_) => discovery.errors.push(DiscoveryError {
                    path: input.clone(),
                    error: anyhow::anyhow!("input is neither a regular file nor a directory"),
                }),
                Err(error) => discovery
                    .errors
                    .push(DiscoveryError { path: input.clone(), error: error.into() }),
            }
        }

        discovery.files.sort();
        discovery.files.dedup();
        discovery
    }

    fn visit_directory(&mut self, directory: &Path, recursive: bool) {
        let read_dir = match fs::read_dir(directory)
            .with_context(|| format!("cannot read directory {}", directory.display()))
        {
            Ok(entries) => entries,
            Err(error) => {
                self.errors.push(DiscoveryError { path: directory.to_path_buf(), error });
                return;
            }
        };

        let mut entries = Vec::new();
        for entry in read_dir {
            match entry {
                Ok(entry) => entries.push(entry),
                Err(error) => self
                    .errors
                    .push(DiscoveryError { path: directory.to_path_buf(), error: error.into() }),
            }
        }
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            match entry.file_type() {
                // Do not follow directory symlinks. Direct file inputs still work because metadata()
                // above follows their target, but discovered links cannot create recursive loops.
                Ok(file_type) if file_type.is_symlink() => {}
                Ok(file_type) if file_type.is_file() && is_ncm(&path) => self.files.push(path),
                Ok(file_type) if file_type.is_dir() && recursive => {
                    self.visit_directory(&path, true)
                }
                Ok(_) => {}
                Err(error) => self.errors.push(DiscoveryError { path, error: error.into() }),
            }
        }
    }
}

fn is_ncm(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ncm"))
}

#[cfg(test)]
mod tests {
    use super::Discovery;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!("ncmc-discovery-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn create(path: &Path) {
        fs::write(path, []).unwrap();
    }

    fn file_names(discovery: Discovery, root: &Path) -> Vec<PathBuf> {
        discovery
            .files
            .into_iter()
            .map(|path| path.strip_prefix(root).unwrap().to_path_buf())
            .collect()
    }

    #[test]
    fn finds_only_top_level_ncm_files_case_insensitively() {
        let temp = TempDir::new();
        create(&temp.path().join("a.ncm"));
        create(&temp.path().join("upper.NCM"));
        create(&temp.path().join("ignore.txt"));
        create(&temp.path().join("空 白 日本語 🎵.ncm"));
        fs::create_dir(temp.path().join("sub")).unwrap();
        create(&temp.path().join("sub").join("b.ncm"));

        let discovery = Discovery::discover(&[temp.path().to_path_buf()], false);
        assert!(discovery.errors.is_empty());
        let files = file_names(discovery, temp.path());
        assert_eq!(files.len(), 3);
        assert!(files.contains(&PathBuf::from("a.ncm")));
        assert!(files.contains(&PathBuf::from("upper.NCM")));
        assert!(files.contains(&PathBuf::from("空 白 日本語 🎵.ncm")));
    }

    #[test]
    fn recursively_finds_nested_ncm_files() {
        let temp = TempDir::new();
        create(&temp.path().join("a.ncm"));
        fs::create_dir(temp.path().join("sub")).unwrap();
        create(&temp.path().join("sub").join("b.ncm"));
        fs::create_dir(temp.path().join("sub").join("nested")).unwrap();
        create(&temp.path().join("sub").join("nested").join("c.NCM"));

        let discovery = Discovery::discover(&[temp.path().to_path_buf()], true);
        assert!(discovery.errors.is_empty());
        assert_eq!(
            file_names(discovery, temp.path()),
            vec![
                PathBuf::from("a.ncm"),
                PathBuf::from("sub/b.ncm"),
                PathBuf::from("sub/nested/c.NCM"),
            ]
        );
    }

    #[test]
    fn mixes_direct_files_with_directories_without_filtering_the_file() {
        let temp = TempDir::new();
        let direct = temp.path().join("extensionless cache");
        create(&direct);
        create(&temp.path().join("from-directory.ncm"));
        create(&temp.path().join("ignored.mp3"));

        let discovery = Discovery::discover(&[direct.clone(), temp.path().to_path_buf()], false);
        assert!(discovery.errors.is_empty());
        assert_eq!(discovery.files.len(), 2);
        assert!(discovery.files.contains(&direct));
        assert!(discovery.files.contains(&temp.path().join("from-directory.ncm")));
    }

    #[test]
    fn reports_missing_inputs_as_discovery_errors() {
        let temp = TempDir::new();
        let missing = temp.path().join("missing.ncm");

        let discovery = Discovery::discover(&[missing.clone()], true);
        assert!(discovery.files.is_empty());
        assert_eq!(discovery.errors.len(), 1);
        assert_eq!(discovery.errors[0].path, missing);
    }

    #[cfg(unix)]
    #[test]
    fn recursive_discovery_does_not_follow_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new();
        fs::create_dir(temp.path().join("real")).unwrap();
        create(&temp.path().join("real").join("song.ncm"));
        symlink(temp.path().join("real"), temp.path().join("linked-real")).unwrap();

        let discovery = Discovery::discover(&[temp.path().to_path_buf()], true);
        assert!(discovery.errors.is_empty());
        assert_eq!(file_names(discovery, temp.path()), vec![PathBuf::from("real/song.ncm")]);
    }
}
