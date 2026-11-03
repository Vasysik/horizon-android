//! Reopen disposable installer inputs, recovering if Android evicted the cache.
use std::{fs::File, io, path::Path};

pub(super) fn open(
    path: &Path,
    mut restore: impl FnMut() -> crate::Result<()>,
) -> crate::Result<File> {
    for attempt in 0..3 {
        match File::open(path) {
            Ok(file) => return Ok(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound && attempt < 2 => {
                restore()?;
            }
            Err(error) => {
                return Err(format!(
                    "Could not open Minecraft installation input {}: {error}",
                    path.display()
                )
                .into());
            }
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Read};

    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "horizon-cache-recovery-{}-{stamp}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn eviction_between_restore_and_open_retries_without_touching_player_data() {
        let home = Scratch::new();
        let cache = home.0.join("cache/objects");
        let path = cache.join("verified-object");
        fs::write(home.0.join("world-save"), b"keep").unwrap();
        let mut restores = 0;
        let mut file = open(&path, || {
            restores += 1;
            fs::create_dir_all(&cache)?;
            fs::write(&path, b"verified payload")?;
            if restores == 1 {
                fs::remove_dir_all(&cache)?;
            }
            Ok(())
        })
        .unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"verified payload");
        assert_eq!(restores, 2);
        assert_eq!(fs::read(home.0.join("world-save")).unwrap(), b"keep");
    }

    #[test]
    fn existing_input_is_not_redownloaded() {
        let home = Scratch::new();
        let path = home.0.join("verified");
        fs::write(&path, b"data").unwrap();
        open(&path, || panic!("existing verified input must be reused")).unwrap();
    }

    #[test]
    fn opened_input_remains_readable_after_its_cache_entry_is_removed() {
        let home = Scratch::new();
        let path = home.0.join("verified");
        fs::write(&path, b"verified bytes").unwrap();
        let mut input = open(&path, || panic!("must already exist")).unwrap();
        fs::remove_file(&path).unwrap();
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"verified bytes");
    }

    #[test]
    fn repeated_eviction_is_bounded_and_identifies_the_input() {
        let home = Scratch::new();
        let path = home.0.join("missing-client.jar");
        let mut attempts = 0;
        let error = open(&path, || {
            attempts += 1;
            Ok(())
        })
        .unwrap_err()
        .to_string();
        assert_eq!(attempts, 2);
        assert!(error.contains("missing-client.jar"));
    }
}
