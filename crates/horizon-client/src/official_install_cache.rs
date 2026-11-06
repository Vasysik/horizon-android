//! Verify opened installer inputs, recovering missing or corrupted cache entries.
use std::{
    fs::File,
    io::{self, Seek},
    path::Path,
};

pub(super) fn open(
    path: &Path,
    mut valid: impl FnMut(&File) -> crate::Result<bool>,
    mut restore: impl FnMut() -> crate::Result<()>,
) -> crate::Result<File> {
    for attempt in 0..3 {
        match File::open(path) {
            Ok(mut file) => {
                // Download verification may be minutes old. Validate this open
                // handle, then rewind it for the ZIP/JSON reader or asset copy.
                if valid(&file)? {
                    file.rewind()?;
                    return Ok(file);
                }
                if attempt == 2 {
                    return Err(format!(
                        "Minecraft installation input failed verification: {}",
                        path.display()
                    )
                    .into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound && attempt < 2 => {}
            Err(error) => {
                return Err(format!(
                    "Could not open Minecraft installation input {}: {error}",
                    path.display()
                )
                .into());
            }
        }
        restore()?;
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Read};

    fn has_bytes(mut file: &File, expected: &[u8]) -> crate::Result<bool> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes == expected)
    }

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
        let mut file = open(
            &path,
            |file| has_bytes(file, b"verified payload"),
            || {
                restores += 1;
                fs::create_dir_all(&cache)?;
                fs::write(&path, b"verified payload")?;
                if restores == 1 {
                    fs::remove_dir_all(&cache)?;
                }
                Ok(())
            },
        )
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
        let mut input = open(
            &path,
            |file| has_bytes(file, b"data"),
            || panic!("existing verified input must be reused"),
        )
        .unwrap();
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(
            bytes, b"data",
            "verification must leave the reader at the start"
        );
    }

    #[test]
    fn opened_input_remains_readable_after_its_cache_entry_is_removed() {
        let home = Scratch::new();
        let path = home.0.join("verified");
        fs::write(&path, b"verified bytes").unwrap();
        let mut input = open(
            &path,
            |file| has_bytes(file, b"verified bytes"),
            || panic!("must already exist"),
        )
        .unwrap();
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
        let error = open(
            &path,
            |_| Ok(true),
            || {
                attempts += 1;
                Ok(())
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(attempts, 2);
        assert!(error.contains("missing-client.jar"));
    }

    #[test]
    fn truncated_archive_is_restored_before_unpacking() {
        use std::io::{Cursor, Write};
        let home = Scratch::new();
        let path = home.0.join("client.jar");
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("assets/test.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"synthetic resource").unwrap();
        let archive = zip.finish().unwrap().into_inner();
        fs::write(&path, &archive[..archive.len() - 22]).unwrap();
        assert!(zip::ZipArchive::new(File::open(&path).unwrap()).is_err());
        let mut restores = 0;
        let file = open(
            &path,
            |file| has_bytes(file, &archive),
            || {
                restores += 1;
                fs::write(&path, &archive)?;
                Ok(())
            },
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let mut text = String::new();
        zip.by_name("assets/test.txt")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "synthetic resource");
        assert_eq!(restores, 1);
    }

    #[test]
    fn changed_bytes_are_restored_even_when_size_matches() {
        let home = Scratch::new();
        let path = home.0.join("object");
        let expected = b"verified object";
        fs::write(home.0.join("world-save"), b"keep").unwrap();
        fs::write(&path, vec![b'x'; expected.len()]).unwrap();
        let mut restores = 0;
        let mut input = open(
            &path,
            |file| has_bytes(file, expected),
            || {
                restores += 1;
                fs::write(&path, expected)?;
                Ok(())
            },
        )
        .unwrap();
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, expected);
        assert_eq!(restores, 1);
        assert_eq!(fs::read(home.0.join("world-save")).unwrap(), b"keep");
    }

    #[test]
    fn repeated_corruption_is_bounded_and_identifies_the_input() {
        let home = Scratch::new();
        let path = home.0.join("corrupt-client.jar");
        fs::write(&path, b"bad").unwrap();
        let mut restores = 0;
        let error = open(
            &path,
            |file| has_bytes(file, b"good"),
            || {
                restores += 1;
                fs::write(&path, b"bad")?;
                Ok(())
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(restores, 2);
        assert!(error.contains("corrupt-client.jar"));
        assert!(error.contains("verification"));
    }
}
