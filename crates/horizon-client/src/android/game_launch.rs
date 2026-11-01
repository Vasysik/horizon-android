//! Frontend-independent game launch parameters. No launcher widgets or config reads.
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct LaunchRequest {
    pub resources: PathBuf,
    pub game_home: PathBuf,
    pub session_file: Option<PathBuf>,
    pub offline_name: String,
    pub server: String,
    pub control_socket: Option<PathBuf>,
    pub extra_arguments: Vec<String>,
    pub open_gl: bool,
    pub gpu_batching: bool,
}
impl LaunchRequest {
    pub fn arguments(&self) -> Vec<String> {
        let mut args = vec![
            "--resources".into(),
            self.resources.to_string_lossy().into_owned(),
            "--game-dir".into(),
            self.game_home.to_string_lossy().into_owned(),
        ];
        if let Some(socket) = &self.control_socket {
            args.extend([
                "--control-socket".into(),
                socket.to_string_lossy().into_owned(),
            ]);
        }
        if let Some(session) = &self.session_file {
            args.extend([
                "--session-file".into(),
                session.to_string_lossy().into_owned(),
            ]);
        } else if !self.offline_name.is_empty() {
            args.extend(["--name".into(), self.offline_name.clone()]);
        }
        if !self.server.is_empty() {
            args.extend(["--server".into(), self.server.clone()]);
        }
        args.extend(self.extra_arguments.iter().cloned());
        args
    }
}
/// The native host owns the window lifecycle; a future frontend supplies this request.
#[cfg(target_os = "android")]
pub(crate) fn start(
    request: LaunchRequest,
) -> horizon_client::Result<Box<dyn winit::application::ApplicationHandler>> {
    if !request.game_home.is_absolute() || !request.resources.is_absolute() {
        return Err("Game and resource directories must be absolute".into());
    }
    let previous = std::env::current_dir()?;
    std::env::set_current_dir(&request.game_home)?;
    horizon_client::android_runtime::configure_graphics(request.open_gl);
    horizon_client::android_runtime::configure_gpu(request.gpu_batching);
    crate::game::android_reset_bundled_mods();
    let result = crate::game::android_game(request.arguments());
    if result.is_err() {
        let _ = std::env::set_current_dir(previous);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> LaunchRequest {
        LaunchRequest {
            resources: "resources with spaces".into(),
            game_home: "profile".into(),
            session_file: None,
            offline_name: "Offline".into(),
            server: "example.test".into(),
            control_socket: None,
            extra_arguments: vec![],
            open_gl: false,
            gpu_batching: false,
        }
    }
    #[test]
    fn session_identity_wins_over_offline_name() {
        let mut r = request();
        r.session_file = Some("private/session.json".into());
        let args = r.arguments();
        assert!(!args.iter().any(|a| a == "--name"));
        assert!(
            args.windows(2)
                .any(|a| a == ["--session-file", "private/session.json"])
        );
    }
    #[test]
    fn paths_are_arguments_not_shell_text_and_snapshot_is_independent() {
        let mut r = request();
        let snapshot = r.arguments();
        r.offline_name = "Changed".into();
        assert_eq!(snapshot[1], "resources with spaces");
        assert!(snapshot.windows(2).any(|a| a == ["--name", "Offline"]));
        assert!(snapshot.windows(2).any(|a| a == ["--game-dir", "profile"]));
        assert!(
            snapshot
                .windows(2)
                .any(|a| a == ["--server", "example.test"])
        );
    }
}
