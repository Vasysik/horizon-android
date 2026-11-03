//! Recovery for a process interrupted before its first Vulkan frame completes.
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
static HOME: OnceLock<PathBuf> = OnceLock::new();
static FILE: Mutex<()> = Mutex::new(());
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MARKER: &str = "vulkan-startup.pending";
pub fn configure(home: PathBuf) {
    let _ = HOME.set(home);
}
pub fn recover(home: &Path) -> bool {
    let _lock = FILE.lock().unwrap_or_else(|e| e.into_inner());
    let path = home.join(MARKER);
    let pending = path.is_file();
    if pending {
        let _ = std::fs::remove_file(path);
    }
    pending
}
pub struct Startup {
    path: PathBuf,
    token: String,
}
impl Startup {
    fn at(home: &Path) -> Option<Self> {
        let _lock = FILE.lock().unwrap_or_else(|e| e.into_inner());
        let path = home.join(MARKER);
        let token = format!(
            "{}:{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        std::fs::write(&path, &token).ok()?;
        Some(Self { path, token })
    }
    pub fn complete(self) {
        let _lock = FILE.lock().unwrap_or_else(|e| e.into_inner());
        // A late completion from the loading renderer cannot clear a newer renderer's marker.
        if std::fs::read_to_string(&self.path).ok().as_deref() == Some(self.token.as_str()) {
            let _ = std::fs::remove_file(self.path);
        }
    }
}
pub fn begin() -> Option<Startup> {
    Startup::at(HOME.get()?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_latest_completed_renderer_clears_recovery() {
        let home = std::env::temp_dir().join(format!("horizon-gpu-startup-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        let old = Startup::at(&home).unwrap();
        let new = Startup::at(&home).unwrap();
        old.complete();
        assert!(home.join(MARKER).is_file());
        new.complete();
        assert!(!recover(&home));
        let _interrupted = Startup::at(&home).unwrap();
        assert!(recover(&home));
        assert!(!recover(&home));
        std::fs::remove_dir_all(home).unwrap();
    }
}
