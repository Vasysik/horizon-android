//! Installing the player's privately prepared resource bundle on a mobile device.
use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Clone, Deserialize, Serialize)]
pub struct Bundle {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

impl Bundle {
    pub fn validate(&self) -> Result<()> {
        if self.version != horizon_core::VANILLA_VERSION {
            return Err(format!(
                "This engine only implements Minecraft {}",
                horizon_core::VANILLA_VERSION
            )
            .into());
        }
        let url = url::Url::parse(&self.url)?;
        let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost"));
        if !(url.scheme() == "https" || url.scheme() == "http" && loopback)
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Resource URL must use HTTPS (HTTP is allowed only on loopback)".into());
        }
        if self.sha256.len() != 64
            || !self.sha256.bytes().all(|c| c.is_ascii_hexdigit())
            || self.size == 0
            || self.size > 4 * 1024 * 1024 * 1024
        {
            return Err("Invalid resource size or SHA-256".into());
        }
        Ok(())
    }
}

pub fn ready(home: &Path) -> bool {
    home.join(".local/horizon-client")
        .join(horizon_core::VANILLA_VERSION)
        .join("horizon-resources.json")
        .is_file()
}

pub fn install(bundle: &Bundle, home: &Path, progress: impl Fn(u64, u64)) -> Result<()> {
    bundle.validate()?;
    let versions = home.join(".local/horizon-client");
    fs::create_dir_all(&versions)?;
    let download = versions.join("download.partial");
    let staging = versions.join("install.partial");
    let result = (|| -> Result<()> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(30))
            .timeout_read(std::time::Duration::from_secs(120))
            .redirects(0)
            .build();
        let mut response = agent.get(&bundle.url).call()?.into_reader();
        let mut file = fs::File::create(&download)?;
        let mut digest = Sha256::new();
        let mut received = 0;
        let mut buffer = [0u8; 65536];
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            received += count as u64;
            if received > bundle.size {
                return Err("Resource download exceeds expected size".into());
            }
            file.write_all(&buffer[..count])?;
            digest.update(&buffer[..count]);
            progress(received, bundle.size);
        }
        file.sync_all()?;
        drop(file);
        let checksum = format!("{:x}", digest.finalize());
        if received != bundle.size || !checksum.eq_ignore_ascii_case(&bundle.sha256) {
            return Err("Resource download checksum or size mismatch".into());
        }
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        fs::create_dir(&staging)?;
        extract(&download, &staging)?;
        if !staging.join("horizon-resources.json").is_file() {
            return Err("Bundle has no Horizon resources manifest".into());
        }
        let target = versions.join(&bundle.version);
        if target.exists() {
            return Err("A resource version already exists; refusing to replace it".into());
        }
        fs::rename(&staging, target)?;
        Ok(())
    })();
    let _ = fs::remove_file(download);
    if staging.exists() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}

pub(crate) fn extract(archive: &Path, destination: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let path = entry
            .enclosed_name()
            .ok_or("Resource archive contains an unsafe path")?;
        if entry.is_symlink() {
            return Err("Resource archive contains a symbolic link".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Resource archive is too large")?;
        if total > 8 * 1024 * 1024 * 1024 {
            return Err("Expanded resource archive exceeds 8 GiB".into());
        }
        let path = destination.join(path);
        if entry.is_dir() {
            fs::create_dir_all(path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(path)?;
            std::io::copy(&mut entry, &mut file)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, net::TcpListener};

    fn fixture() -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "horizon-resources.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"{}").unwrap();
        zip.start_file(
            "assets/example.txt",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"private resource").unwrap();
        zip.finish().unwrap().into_inner()
    }

    fn serve(bytes: Vec<u8>) -> (Bundle, std::thread::JoinHandle<()>) {
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        let bundle = Bundle {
            version: horizon_core::VANILLA_VERSION.into(),
            url: format!("http://{}/resources.zip", server.local_addr().unwrap()),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size: bytes.len() as u64,
        };
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = server.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 4096];
            stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        (bundle, handle)
    }

    fn temporary(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "horizon-install-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn download_is_verified_before_resources_become_ready() {
        let home = temporary("valid");
        let (bundle, server) = serve(fixture());
        assert!(!ready(&home));
        install(&bundle, &home, |_, _| {}).unwrap();
        server.join().unwrap();
        assert!(ready(&home));
        assert_eq!(
            fs::read(
                home.join(".local/horizon-client")
                    .join(&bundle.version)
                    .join("assets/example.txt")
            )
            .unwrap(),
            b"private resource"
        );
        assert!(!home.join(".local/horizon-client/download.partial").exists());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn a_corrupt_download_never_becomes_an_installed_version() {
        let home = temporary("corrupt");
        let (mut bundle, server) = serve(fixture());
        bundle.sha256 = "0".repeat(64);
        assert!(install(&bundle, &home, |_, _| {}).is_err());
        server.join().unwrap();
        assert!(!ready(&home));
        assert!(!home.join(".local/horizon-client/download.partial").exists());
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn unsupported_versions_and_insecure_remote_urls_are_rejected() {
        let mut bundle = Bundle {
            version: "1.21".into(),
            url: "https://example.com/resources.zip".into(),
            sha256: "a".repeat(64),
            size: 100,
        };
        assert!(bundle.validate().is_err());
        bundle.version = horizon_core::VANILLA_VERSION.into();
        assert!(bundle.validate().is_ok());
        bundle.url = "http://example.com/resources.zip".into();
        assert!(bundle.validate().is_err());
        bundle.url = "http://127.0.0.1:8123/26.3.zip".into();
        assert!(bundle.validate().is_ok());
        bundle.url = "http://localhost:80@example.com/resources.zip".into();
        assert!(bundle.validate().is_err());
    }
}
