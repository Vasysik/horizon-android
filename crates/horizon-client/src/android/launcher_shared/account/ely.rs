//! Ely.by's documented Yggdrasil API. Passwords are used for one request only;
//! the existing SecretStore holds the access token and launcher client token.
//! https://docs.ely.by/en/minecraft-auth.html
use super::*;

const AUTH: &str = "https://authserver.ely.by";
pub(super) const REFUSED: &str =
    "Ely.by sign-in was refused. Check your credentials or sign in again.";

fn exchange(endpoint: &str, body: Value) -> Result<Value, String> {
    let (status, value) = send(
        request_to("POST", endpoint),
        Some(Body::Json(body)),
        "Ely.by",
    )?;
    match status {
        200 => Ok(value),
        401 if value["errorMessage"] == "Account protected with two factor auth." => {
            Err("Enter your Ely.by two-factor code and try again.".into())
        }
        400 | 401 | 403 => Err(REFUSED.into()),
        429 => Err("Ely.by is receiving too many requests. Try again later.".into()),
        _ => Err("Ely.by is unavailable. Try again later.".into()),
    }
}

fn decode(value: &Value, client: &str) -> Result<(Account, Secrets), String> {
    let profile = &value["selectedProfile"];
    let uuid = str_field(profile, "id")
        .unwrap_or_default()
        .replace('-', "")
        .to_lowercase();
    let name = str_field(profile, "name").unwrap_or_default();
    if uuid.len() != 32
        || !uuid.bytes().all(|b| b.is_ascii_hexdigit())
        || !(1..=16).contains(&name.len())
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Ely.by returned an invalid Minecraft profile.".into());
    }
    let token = str_field(value, "accessToken")
        .filter(|t| !t.is_empty())
        .ok_or("Ely.by returned no game session.")?;
    if value
        .get("clientToken")
        .and_then(Value::as_str)
        .is_some_and(|t| t != client)
    {
        return Err("Ely.by returned a different launcher identity.".into());
    }
    Ok((
        Account {
            provider: Provider::Ely,
            uuid,
            name,
            skin: None,
            cape: None,
        },
        Secrets {
            provider: Provider::Ely,
            refresh_token: client.into(),
            sign_in_required: false,
            minecraft_token: token,
            minecraft_expires_at: now() + 3600,
            xuid: None,
            owns: false,
        },
    ))
}

/// Direct login supports Ely.by's documented password:OTP convention. Neither
/// password nor OTP is persisted, included in errors, or passed to the game.
pub fn authenticate(
    username: &str,
    password: &str,
    otp: &str,
) -> Result<(Account, Secrets), String> {
    let (mut account, secrets) = authenticate_with(AUTH, username, password, otp)?;
    update_textures(&mut account);
    Ok((account, secrets))
}
fn authenticate_with(
    base: &str,
    username: &str,
    password: &str,
    otp: &str,
) -> Result<(Account, Secrets), String> {
    if username.trim().is_empty() || password.is_empty() {
        return Err("Enter your Ely.by username and password.".into());
    }
    if !otp.is_empty() && (otp.len() != 6 || !otp.bytes().all(|b| b.is_ascii_digit())) {
        return Err("The two-factor code must contain six digits.".into());
    }
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|_| "Could not create a launcher identity.")?;
    let client: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let password = if otp.is_empty() {
        password.to_owned()
    } else {
        format!("{password}:{otp}")
    };
    let value = exchange(
        &format!("{base}/auth/authenticate"),
        json!({
            "username": username.trim(), "password": password, "clientToken": client, "requestUser": true,
        }),
    )?;
    decode(&value, &client)
}

pub(super) fn refresh(account: &Account, secrets: &Secrets) -> Result<(Account, Secrets), String> {
    let (mut account, secrets) = refresh_with(AUTH, account, secrets)?;
    update_textures(&mut account);
    Ok((account, secrets))
}
fn refresh_with(
    base: &str,
    account: &Account,
    secrets: &Secrets,
) -> Result<(Account, Secrets), String> {
    let value = exchange(
        &format!("{base}/auth/refresh"),
        json!({
            "accessToken": secrets.minecraft_token, "clientToken": secrets.refresh_token,
            "selectedProfile": {"id": account.uuid, "name": account.name},
        }),
    )?;
    let (renewed, secrets) = decode(&value, &secrets.refresh_token)?;
    if renewed.uuid != account.uuid {
        return Err("Ely.by returned another profile. Sign in again.".into());
    }
    Ok((renewed, secrets))
}

fn update_textures(account: &mut Account) {
    let url = format!("https://skinsystem.ely.by/textures/{}", account.name);
    if let Ok((200, value)) = send(request_to("GET", &url), None, "Ely.by skins") {
        account.skin = str_field(&value["SKIN"], "url").map(|url| {
            let model = if value["SKIN"]["metadata"]["model"] == "slim" {
                "slim"
            } else {
                "classic"
            };
            (url, model.into())
        });
        account.cape = str_field(&value["CAPE"], "url");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticate_two_factor_and_refresh_use_only_the_ely_protocol() {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for index in 0..4 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                assert!(line.starts_with(if index < 2 {
                    "POST /auth/authenticate "
                } else {
                    "POST /auth/refresh "
                }));
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" {
                        break;
                    }
                    assert!(!header.to_ascii_lowercase().starts_with("authorization:"));
                    if let Some((name, value)) = header.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse::<usize>().unwrap();
                        }
                    }
                }
                let mut bytes = vec![0; length];
                reader.read_exact(&mut bytes).unwrap();
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                let (status, value) = if index == 0 {
                    assert_eq!(request["password"], "secret");
                    (
                        401,
                        json!({"errorMessage":"Account protected with two factor auth."}),
                    )
                } else if index == 3 {
                    (
                        401,
                        json!({"errorMessage":"Token expired. private-server-text"}),
                    )
                } else {
                    if index == 1 {
                        assert_eq!(request["password"], "secret:123456");
                        assert_eq!(request["username"], "Player");
                    } else {
                        assert_eq!(request["accessToken"], "access-1");
                        assert!(request.get("password").is_none());
                    }
                    (
                        200,
                        json!({"clientToken":request["clientToken"],"accessToken":format!("access-{index}"),"selectedProfile":{"id":"0123456789abcdef0123456789abcdef","name":"Player"}}),
                    )
                };
                let body = value.to_string();
                write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        assert!(authenticate_with(&base, "Player", "secret", "")
            .unwrap_err()
            .contains("two-factor"));
        let (account, secrets) = authenticate_with(&base, "Player", "secret", "123456").unwrap();
        let (_, renewed) = refresh_with(&base, &account, &secrets).unwrap();
        assert_eq!(renewed.game_token(), "access-2");
        assert_eq!(
            refresh_with(&base, &account, &renewed).unwrap_err(),
            REFUSED
        );
        server.join().unwrap();
    }
    #[test]
    fn provider_identity_is_distinct_and_secrets_are_redacted() {
        let value = json!({"accessToken":"private-access", "clientToken":"private-client",
            "selectedProfile":{"id":"0123456789abcdef0123456789abcdef", "name":"Player"}});
        let (ely, secrets) = decode(&value, "private-client").unwrap();
        let mut microsoft = ely.clone();
        microsoft.provider = Provider::Microsoft;
        assert_ne!(ely.key(), microsoft.key());
        assert!(!secrets.owns);
        assert!(!format!("{secrets:?}").contains("private"));
        assert!(!serde_json::to_string(&ely).unwrap().contains("private"));
        assert!(decode(&value, "another-client").is_err());
        assert!(decode(
            &json!({"accessToken":"x","selectedProfile":{"id":"../bad","name":"Player"}}),
            "c"
        )
        .is_err());
    }
    #[test]
    fn both_providers_keep_distinct_credentials_and_ely_never_grants_ownership() {
        let value = json!({"accessToken":"ely-token","clientToken":"client",
            "selectedProfile":{"id":"0123456789abcdef0123456789abcdef","name":"Player"}});
        let (ely, ely_secrets) = decode(&value, "client").unwrap();
        let mut microsoft = ely.clone();
        microsoft.provider = Provider::Microsoft;
        let microsoft_secrets = Secrets {
            provider: Provider::Microsoft,
            refresh_token: "ms-refresh".into(),
            minecraft_token: "ms-token".into(),
            minecraft_expires_at: now() + 7200,
            sign_in_required: false,
            xuid: None,
            owns: true,
        };
        let store = MemoryStore::default();
        assert!(fresh(
            &Endpoints::live(),
            "unused",
            &microsoft,
            ely_secrets.clone()
        )
        .is_err());
        assert!(fresh(
            &Endpoints::live(),
            "unused",
            &ely,
            microsoft_secrets.clone()
        )
        .is_err());
        let mut accounts = Accounts::default();
        accounts.add(&store, ely.clone(), &ely_secrets).unwrap();
        assert_eq!(accounts.selected(), Some(&ely));
        assert!(!accounts.has_owner(&store));
        assert_eq!(
            session::status(&store, &ely.key()).state,
            session::State::NeedsRefresh
        );
        accounts
            .add(&store, microsoft.clone(), &microsoft_secrets)
            .unwrap();
        assert_eq!(accounts.accounts.len(), 2);
        assert_eq!(
            accounts.find(&microsoft.key()).unwrap().provider,
            Provider::Microsoft
        );
        assert_eq!(accounts.find(&ely.key()).unwrap().provider, Provider::Ely);
        assert!(accounts.has_owner(&store));
        assert!(store
            .get(&ely.key())
            .unwrap()
            .unwrap()
            .contains("ely-token"));
        assert!(store
            .get(&microsoft.key())
            .unwrap()
            .unwrap()
            .contains("ms-token"));
        accounts.remove(&store, &ely.key()).unwrap();
        assert_eq!(accounts.selected(), Some(&microsoft));
        assert!(store.get(&microsoft.key()).unwrap().is_some());
    }
}
