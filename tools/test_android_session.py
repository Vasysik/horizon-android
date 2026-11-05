#!/usr/bin/env python3
"""Exercise the real session owner without loading Minecraft or creating a GPU."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    root = ROOT / '.local/android/session-tests'
    (root / 'src').mkdir(parents=True, exist_ok=True)
    (root / 'Cargo.toml').write_text('''[package]
name = "android-session-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
self_cell = "1.3"
''', encoding='utf-8')
    (root / 'src/lib.rs').write_text('''#![allow(dead_code)]
#[cfg(test)] use std::sync::Arc;
struct Resources(String);
mod split {
    pub(super) struct Split<'a>(pub &'a Arc<super::Resources>);
    use std::sync::Arc;
}
#[path = "../../../../crates/horizon-client/src/game/session.rs"] mod session;
#[test]
fn relaunch_releases_each_sessions_resources() {
    for n in 0..8 {
        let owner = Arc::new(Resources(n.to_string()));
        let weak = Arc::downgrade(&owner);
        let mut session = session::Session::new(owner, |owner| Some(split::Split(owner)));
        assert!(session.has_game());
        session.with_game(|game| assert_eq!(game.0.0, n.to_string()));
        assert!(weak.upgrade().is_some());
        drop(session);
        assert!(weak.upgrade().is_none(), "closed session retained resources");
    }
}
#[test]
fn failed_and_cancelled_construction_release_resources() {
    let owner = Arc::new(Resources("failed".into()));
    let weak = Arc::downgrade(&owner);
    let failed = session::Session::try_new(owner, |_| Err::<Option<split::Split<'_>>, _>("failed"));
    assert!(failed.is_err());
    assert!(weak.upgrade().is_none());
    let owner = Arc::new(Resources("cancelled".into()));
    let weak = Arc::downgrade(&owner);
    let cancelled = session::Session::new(owner, |_| None);
    assert!(!cancelled.has_game());
    drop(cancelled);
    assert!(weak.upgrade().is_none());
}
''', encoding='utf-8')
    subprocess.run(['cargo', 'test', '--manifest-path', str(root / 'Cargo.toml')],
                   cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
