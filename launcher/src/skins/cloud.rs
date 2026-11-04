//! Provider-specific cloud operations. Local selection never calls these APIs.
use super::{read_cape, read_skin, Model, MAX_IMPORT_BYTES};
use crate::account::{Account, Provider, Secrets};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
const PROFILE: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Texture {
    pub id: String,
    pub title: String,
    pub url: String,
    pub active: bool,
    pub slim: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub skins: Vec<Texture>,
    pub capes: Vec<Texture>,
}
#[derive(Default, Clone)]
pub struct Cache {
    pub snapshot: Snapshot,
    pub skin: Option<PathBuf>,
    pub cape: Option<PathBuf>,
    pub slim: bool,
}

fn official_snapshot(value: &Value) -> Snapshot {
    let list = |key: &str| {
        value[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| {
                Some(Texture {
                    id: v["id"].as_str()?.into(),
                    title: v["alias"]
                        .as_str()
                        .unwrap_or(if key == "skins" {
                            "Current Microsoft skin"
                        } else {
                            "Official cape"
                        })
                        .into(),
                    url: v["url"].as_str()?.into(),
                    active: v["state"] == "ACTIVE",
                    slim: v["variant"]
                        .as_str()
                        .is_some_and(|s| s.eq_ignore_ascii_case("slim")),
                })
            })
            .collect()
    };
    Snapshot {
        skins: list("skins"),
        capes: list("capes"),
    }
}
fn response(result: Result<ureq::Response, ureq::Error>) -> Result<Value, String> {
    match result {
        Ok(r) => {
            let mut bytes = Vec::new();
            r.into_reader()
                .take(1024 * 1024)
                .read_to_end(&mut bytes)
                .map_err(|_| "Could not read the cloud profile.")?;
            if bytes.is_empty() {
                Ok(Value::Null)
            } else {
                serde_json::from_slice(&bytes).map_err(|_| "Invalid cloud profile response.".into())
            }
        }
        Err(ureq::Error::Status(401 | 403, _)) => {
            Err("Sign in again before changing your cloud appearance.".into())
        }
        Err(ureq::Error::Status(429, _)) => {
            Err("The profile service is busy. Try again later.".into())
        }
        Err(_) => Err(
            "Could not contact the profile service. Your local gallery is still available.".into(),
        ),
    }
}
pub fn fetch(account: &Account, secrets: &Secrets, root: &Path) -> Result<Cache, String> {
    if account.provider != secrets.provider() {
        return Err("Account and session providers do not match.".into());
    }
    let snapshot = match account.provider {
        Provider::Microsoft => official_snapshot(&response(
            crate::account::request_to("GET", PROFILE)
                .set("Authorization", &format!("Bearer {}", secrets.game_token()))
                .call(),
        )?),
        Provider::Ely => {
            let value = response(
                crate::account::request_to(
                    "GET",
                    &format!("https://skinsystem.ely.by/textures/{}", account.name),
                )
                .call(),
            )?;
            let texture = |key: &str| {
                value[key]["url"].as_str().map(|url| Texture {
                    id: format!("ely-{key}"),
                    title: format!("Current Ely.by {}", key.to_lowercase()),
                    url: url.into(),
                    active: true,
                    slim: value[key]["metadata"]["model"] == "slim",
                })
            };
            Snapshot {
                skins: texture("SKIN").into_iter().collect(),
                capes: texture("CAPE").into_iter().collect(),
            }
        }
    };
    let skin = snapshot
        .skins
        .iter()
        .find(|t| t.active)
        .map(|t| download(root, &t.url, false))
        .transpose()?;
    let cape = snapshot
        .capes
        .iter()
        .find(|t| t.active)
        .map(|t| download(root, &t.url, true))
        .transpose()?;
    let slim = snapshot.skins.iter().any(|t| t.active && t.slim);
    Ok(Cache {
        snapshot,
        skin,
        cape,
        slim,
    })
}

/// Only known provider image hosts; credentials are never attached to downloads.
fn texture_url(text: &str) -> Result<String, String> {
    let mut url = url::Url::parse(text).map_err(|_| "Invalid cloud texture URL.")?;
    let allowed = matches!(
        url.host_str(),
        Some("textures.minecraft.net" | "ely.by" | "skinsystem.ely.by")
    );
    if !allowed
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !matches!(url.scheme(), "http" | "https")
    {
        return Err("The provider returned an unsupported texture host.".into());
    }
    url.set_scheme("https")
        .map_err(|_| "Invalid texture URL.")?;
    Ok(url.into())
}
pub fn download(root: &Path, url: &str, cape: bool) -> Result<PathBuf, String> {
    let url = texture_url(url)?;
    let response = crate::account::request_to("GET", &url)
        .call()
        .map_err(|_| "Could not download the cloud texture.")?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(MAX_IMPORT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the cloud texture.")?;
    let image = if cape {
        read_cape(&bytes)?
    } else {
        read_skin(&bytes)?
    };
    let mut png = std::io::Cursor::new(vec![]);
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| "Could not cache the cloud texture.")?;
    let dir = root.join("cloud");
    std::fs::create_dir_all(&dir).map_err(|_| "Could not create the cloud cache.")?;
    let path = dir.join(format!("{:x}.png", Sha256::digest(png.get_ref())));
    std::fs::write(&path, png.get_ref()).map_err(|_| "Could not cache the cloud texture.")?;
    Ok(path)
}
pub fn upload_skin(
    account: &Account,
    secrets: &Secrets,
    bytes: &[u8],
    model: Model,
) -> Result<(), String> {
    upload_skin_at(PROFILE, account, secrets, bytes, model)
}
fn upload_skin_at(
    profile: &str,
    account: &Account,
    secrets: &Secrets,
    bytes: &[u8],
    model: Model,
) -> Result<(), String> {
    if account.provider != secrets.provider() {
        return Err("Account and session providers do not match.".into());
    }
    if account.provider != Provider::Microsoft {
        return Err("Ely.by cloud skins are managed on ely.by; its public API does not expose skin uploads.".into());
    }
    let image = read_skin(bytes)?;
    let mut png = std::io::Cursor::new(vec![]);
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| "Could not encode skin.")?;
    let boundary = "horizon-skin-upload-v1";
    let mut body=format!("--{boundary}\r\nContent-Disposition: form-data; name=\"variant\"\r\n\r\n{}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n",model.name()).into_bytes();
    body.extend_from_slice(png.get_ref());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    response(
        crate::account::request_to("POST", &format!("{profile}/skins"))
            .set("Authorization", &format!("Bearer {}", secrets.game_token()))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .send_bytes(&body),
    )?;
    Ok(())
}
pub fn activate_cape(account: &Account, secrets: &Secrets, id: Option<&str>) -> Result<(), String> {
    activate_cape_at(PROFILE, account, secrets, id)
}
fn activate_cape_at(
    profile: &str,
    account: &Account,
    secrets: &Secrets,
    id: Option<&str>,
) -> Result<(), String> {
    if account.provider != secrets.provider() {
        return Err("Account and session providers do not match.".into());
    }
    if account.provider != Provider::Microsoft {
        return Err(
            "Ely.by does not offer user-selectable cloud capes. Choose a local cape instead."
                .into(),
        );
    }
    if let Some(id) = id {
        // Re-fetch ownership immediately before activation. Never offer unowned official capes.
        let value = response(
            crate::account::request_to("GET", profile)
                .set("Authorization", &format!("Bearer {}", secrets.game_token()))
                .call(),
        )?;
        if !official_snapshot(&value).capes.iter().any(|c| c.id == id) {
            return Err("This cape does not belong to the signed-in account.".into());
        }
        response(
            crate::account::request_to("PUT", &format!("{profile}/capes/active"))
                .set("Authorization", &format!("Bearer {}", secrets.game_token()))
                .send_json(serde_json::json!({"capeId":id})),
        )?;
    } else {
        response(
            crate::account::request_to("DELETE", &format!("{profile}/capes/active"))
                .set("Authorization", &format!("Bearer {}", secrets.game_token()))
                .call(),
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloud_mutations_send_png_variant_and_only_activate_an_owned_cape() {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let profile = format!("http://{}/profile", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for index in 0..4 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let mut length = 0;
                let mut authorization = false;
                let mut multipart = false;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" {
                        break;
                    }
                    let lower = header.to_ascii_lowercase();
                    authorization |= lower.trim() == "authorization: bearer fake-microsoft-token";
                    multipart |= lower.contains("multipart/form-data; boundary=");
                    if let Some((name, value)) = header.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse::<usize>().unwrap();
                        }
                    }
                }
                assert!(authorization);
                let mut bytes = vec![0; length];
                reader.read_exact(&mut bytes).unwrap();
                let (status, body) = match index {
                    0 => {
                        assert!(first.starts_with("POST /profile/skins "));
                        assert!(multipart);
                        assert!(bytes.windows(8).any(|w| w == b"\x89PNG\r\n\x1a\n"));
                        assert!(String::from_utf8_lossy(&bytes).contains("\r\nslim\r\n"));
                        (204, String::new())
                    }
                    2 => {
                        assert!(first.starts_with("PUT /profile/capes/active "));
                        assert_eq!(
                            serde_json::from_slice::<Value>(&bytes).unwrap()["capeId"],
                            "owned"
                        );
                        (204, String::new())
                    }
                    _ => {
                        assert!(first.starts_with("GET /profile "));
                        (200,serde_json::json!({"capes":[{"id":"owned","alias":"Owned cape","url":"https://textures.minecraft.net/texture/abc","state":"INACTIVE"}]}).to_string())
                    }
                };
                write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let account: Account = serde_json::from_value(
            serde_json::json!({"uuid":"0123456789abcdef0123456789abcdef","name":"Player"}),
        )
        .unwrap();
        let secrets:Secrets=serde_json::from_value(serde_json::json!({"refresh_token":"fake-refresh","minecraft_token":"fake-microsoft-token","minecraft_expires_at":9999999999u64,"xuid":null,"owns":true})).unwrap();
        let mut png = std::io::Cursor::new(vec![]);
        super::super::preview::mannequin()
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        upload_skin_at(&profile, &account, &secrets, png.get_ref(), Model::Slim).unwrap();
        activate_cape_at(&profile, &account, &secrets, Some("owned")).unwrap();
        assert!(
            activate_cape_at(&profile, &account, &secrets, Some("unowned"))
                .unwrap_err()
                .contains("does not belong")
        );
        server.join().unwrap();
    }
    #[test]
    fn snapshots_keep_owned_capes_and_reject_credential_urls() {
        let profile = serde_json::json!({"skins":[{"id":"skin","url":"https://textures.minecraft.net/texture/abc","state":"ACTIVE","variant":"SLIM"}],"capes":[{"id":"owned","url":"https://textures.minecraft.net/texture/def","alias":"Mine","state":"INACTIVE"}]});
        let snapshot = official_snapshot(&profile);
        assert!(snapshot.skins[0].slim);
        assert_eq!(snapshot.capes[0].id, "owned");
        assert!(!snapshot.capes[0].active);
        assert!(texture_url("https://textures.minecraft.net.attacker.test/x").is_err());
        assert!(texture_url("https://token@ely.by/x").is_err());
        assert!(texture_url("file:///private").is_err());
        assert_eq!(
            texture_url("http://ely.by/storage/skin.png").unwrap(),
            "https://ely.by/storage/skin.png"
        );
    }
}
