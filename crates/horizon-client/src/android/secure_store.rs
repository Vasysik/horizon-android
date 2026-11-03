//! Android Keystore AES/GCM storage. Ciphertext only is written to app-private files.
use crate::account::SecretStore;
use jni::{
    JavaVM,
    objects::{JByteArray, JObject, JValue},
};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
static VM: OnceLock<JavaVM> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());
const ALIAS: &str = "dev.horizon.android.accounts.v1";
pub fn initialize(vm: JavaVM) {
    let _ = VM.set(vm);
}
pub struct AndroidStore;
fn path(key: &str) -> Result<PathBuf, String> {
    if key.is_empty() || !key.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-') {
        return Err("Invalid account key".into());
    }
    Ok(crate::backend::data_dir()
        .join("account-secrets")
        .join(format!("{key}.gcm")))
}
fn crypt(input: &[u8], decrypt: bool, account: &str) -> Result<Vec<u8>, String> {
    let vm = VM.get().ok_or("Android Keystore is not initialized")?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|_| "Cannot attach Android Keystore")?;
    let result = env.with_local_frame(48, |e| -> jni::errors::Result<Vec<u8>> {
        let provider = JObject::from(e.new_string("AndroidKeyStore")?);
        let alias = JObject::from(e.new_string(ALIAS)?);
        let ks = e
            .call_static_method(
                "java/security/KeyStore",
                "getInstance",
                "(Ljava/lang/String;)Ljava/security/KeyStore;",
                &[JValue::Object(&provider)],
            )?
            .l()?;
        e.call_method(
            &ks,
            "load",
            "(Ljava/security/KeyStore$LoadStoreParameter;)V",
            &[JValue::Object(&JObject::null())],
        )?;
        let exists = e
            .call_method(
                &ks,
                "containsAlias",
                "(Ljava/lang/String;)Z",
                &[JValue::Object(&alias)],
            )?
            .z()?;
        if !exists && !decrypt {
            let aes = JObject::from(e.new_string("AES")?);
            let generator = e
                .call_static_method(
                    "javax/crypto/KeyGenerator",
                    "getInstance",
                    "(Ljava/lang/String;Ljava/lang/String;)Ljavax/crypto/KeyGenerator;",
                    &[JValue::Object(&aes), JValue::Object(&provider)],
                )?
                .l()?;
            let builder = e.new_object(
                "android/security/keystore/KeyGenParameterSpec$Builder",
                "(Ljava/lang/String;I)V",
                &[JValue::Object(&alias), JValue::Int(3)],
            )?;
            for (method, value) in [
                ("setBlockModes", "GCM"),
                ("setEncryptionPaddings", "NoPadding"),
            ] {
                let array = e.new_object_array(1, "java/lang/String", JObject::null())?;
                let value = e.new_string(value)?;
                e.set_object_array_element(&array, 0, value)?;
                e.call_method(
                    &builder,
                    method,
                    "([Ljava/lang/String;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                    &[JValue::Object(array.as_ref())],
                )?;
            }
            e.call_method(
                &builder,
                "setKeySize",
                "(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                &[JValue::Int(256)],
            )?;
            let spec = e
                .call_method(
                    &builder,
                    "build",
                    "()Landroid/security/keystore/KeyGenParameterSpec;",
                    &[],
                )?
                .l()?;
            e.call_method(
                &generator,
                "init",
                "(Ljava/security/spec/AlgorithmParameterSpec;)V",
                &[JValue::Object(&spec)],
            )?;
            e.call_method(&generator, "generateKey", "()Ljavax/crypto/SecretKey;", &[])?;
        }
        let key = e
            .call_method(
                &ks,
                "getKey",
                "(Ljava/lang/String;[C)Ljava/security/Key;",
                &[JValue::Object(&alias), JValue::Object(&JObject::null())],
            )?
            .l()?;
        let algorithm = JObject::from(e.new_string("AES/GCM/NoPadding")?);
        let cipher = e
            .call_static_method(
                "javax/crypto/Cipher",
                "getInstance",
                "(Ljava/lang/String;)Ljavax/crypto/Cipher;",
                &[JValue::Object(&algorithm)],
            )?
            .l()?;
        let data = if decrypt {
            if input.len() < 29 || input[0] != 1 {
                return Err(jni::errors::Error::NullPtr("Invalid encrypted account"));
            }
            let iv = e.byte_array_from_slice(&input[1..13])?;
            let spec = e.new_object(
                "javax/crypto/spec/GCMParameterSpec",
                "(I[B)V",
                &[JValue::Int(128), JValue::Object(iv.as_ref())],
            )?;
            e.call_method(
                &cipher,
                "init",
                "(ILjava/security/Key;Ljava/security/spec/AlgorithmParameterSpec;)V",
                &[JValue::Int(2), JValue::Object(&key), JValue::Object(&spec)],
            )?;
            &input[13..]
        } else {
            e.call_method(
                &cipher,
                "init",
                "(ILjava/security/Key;)V",
                &[JValue::Int(1), JValue::Object(&key)],
            )?;
            input
        };
        let aad = e.byte_array_from_slice(account.as_bytes())?;
        e.call_method(
            &cipher,
            "updateAAD",
            "([B)V",
            &[JValue::Object(aad.as_ref())],
        )?;
        let bytes = e.byte_array_from_slice(data)?;
        let encrypted = e
            .call_method(
                &cipher,
                "doFinal",
                "([B)[B",
                &[JValue::Object(bytes.as_ref())],
            )?
            .l()?;
        let data = e.convert_byte_array(JByteArray::from(encrypted))?;
        if decrypt {
            return Ok(data);
        }
        let iv = e.call_method(&cipher, "getIV", "()[B", &[])?.l()?;
        let iv = e.convert_byte_array(JByteArray::from(iv))?;
        if iv.len() != 12 {
            return Err(jni::errors::Error::NullPtr("Invalid GCM IV"));
        }
        let mut out = vec![1];
        out.extend(iv);
        out.extend(data);
        Ok(out)
    });
    if result.is_err() {
        let _ = env.exception_clear();
    }
    result.map_err(|_| "Cannot read or save this account in Android Keystore. Sign in again if app data was restored on another device.".into())
}
impl SecretStore for AndroidStore {
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let _guard = LOCK.lock().map_err(|_| "Account storage unavailable")?;
        let bytes = match std::fs::read(path(key)?) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Cannot read saved account".into()),
        };
        String::from_utf8(crypt(&bytes, true, key)?)
            .map(Some)
            .map_err(|_| "Invalid saved account".into())
    }
    fn set(&self, key: &str, secret: &str) -> Result<(), String> {
        use std::{io::Write, os::unix::fs::OpenOptionsExt};
        let _guard = LOCK.lock().map_err(|_| "Account storage unavailable")?;
        let path = path(key)?;
        let bytes = crypt(secret.as_bytes(), false, key)?;
        std::fs::create_dir_all(path.parent().unwrap())
            .map_err(|_| "Cannot create account storage")?;
        let tmp = path.with_extension("tmp");
        let result = (|| -> std::io::Result<()> {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)?;
            f.write_all(&bytes)?;
            f.sync_all()?;
            std::fs::rename(&tmp, &path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result.map_err(|_| "Cannot save account".into())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        let _guard = LOCK.lock().map_err(|_| "Account storage unavailable")?;
        match std::fs::remove_file(path(key)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Cannot remove saved account".into()),
        }
    }
    fn persistent(&self) -> bool {
        true
    }
}

/// Development APK only: an isolated, synthetic credential exercises the real
/// JNI implementation over two Android process launches. No login is involved.
#[cfg(debug_assertions)]
pub fn development_probe() -> bool {
    let home = crate::backend::data_dir();
    let request = home.join("keystore-probe-request.json");
    let Ok(phase) = std::fs::read_to_string(&request) else {
        return false;
    };
    let _ = std::fs::remove_file(request);
    let result = std::thread::spawn(move || -> Result<(), String> {
        let key = "00000000000000000000000000000001";
        let other = "00000000000000000000000000000002";
        let secret = "synthetic-keystore-probe-not-a-real-token";
        let store = AndroidStore;
        if phase.trim() == "write" {
            store.set(key, secret)?;
            let original = std::fs::read(path(key)?).map_err(|_| "Probe read failed")?;
            if original
                .windows(secret.len())
                .any(|w| w == secret.as_bytes())
            {
                return Err("Probe plaintext found".into());
            }
            if store.get(key)?.as_deref() != Some(secret) {
                return Err("Probe roundtrip failed".into());
            }
            let mut changed = original.clone();
            *changed.last_mut().unwrap() ^= 1;
            std::fs::write(path(key)?, changed).map_err(|_| "Probe write failed")?;
            if store.get(key).is_ok() {
                return Err("Probe accepted tampering".into());
            }
            std::fs::write(path(key)?, &original).map_err(|_| "Probe restore failed")?;
            std::fs::write(path(other)?, &original).map_err(|_| "Probe substitution failed")?;
            if store.get(other).is_ok() {
                return Err("Probe accepted wrong account".into());
            }
            store.delete(other)?;
        } else if phase.trim() == "read" {
            if store.get(key)?.as_deref() != Some(secret) {
                return Err("Probe restart persistence failed".into());
            }
            store.delete(key)?;
            if store.get(key)?.is_some() {
                return Err("Probe signout failed".into());
            }
        } else {
            return Err("Unknown probe phase".into());
        }
        Ok(())
    })
    .join()
    .unwrap_or_else(|_| Err("Probe worker failed".into()));
    let report = serde_json::json!({"ok":result.is_ok(),"error":result.err()});
    let _ = std::fs::write(home.join("keystore-probe-result.json"), report.to_string());
    true
}
