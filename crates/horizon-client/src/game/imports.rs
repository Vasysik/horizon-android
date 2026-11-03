//! Files handed to the game from outside its folder (Android's "Open with", share and file picker):
//! the inbox installed on a worker (horizon_client::import), with a note over every screen while
//! it works and for a while after.
use horizon_client::{
    hud::{HudAtlas, HudBatch},
    import::{self, Event, Inbox, Kind, Places},
    ui_theme::{Face, Ui, color},
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const POLL: Duration = Duration::from_millis(400);
/// How long a finished import's note stays.
const SHOWN: Duration = Duration::from_secs(7);

struct Note {
    text: String,
    error: bool,
    /// When the import ended; None while it runs.
    ended: Option<Instant>,
}

/// The inbox and its note: held by the loading screen and the game alike.
#[derive(Default)]
pub(super) struct Imports {
    inbox: Option<Inbox>,
    polled: Option<Instant>,
    note: Option<Note>,
    /// A skin the inbox just put in `skins/`, for the game to put on.
    skin: Option<PathBuf>,
}

impl Imports {
    /// Look at the inbox where a platform fills it (Android); the kind of an import that just
    /// finished well. `resources` is the prepared resources' folder (the vanilla root).
    pub(super) fn poll(&mut self, resources: &Path) -> Option<Kind> {
        if !horizon_client::platform::registered()
            || self.polled.is_some_and(|t| t.elapsed() < POLL)
        {
            return None;
        }
        self.polled = Some(Instant::now());
        let inbox = self.inbox.get_or_insert_with(|| {
            Inbox::new(
                import::inbox_dir(),
                Places {
                    game: horizon_client::logging::game_home(),
                    resources: resources.to_path_buf(),
                },
            )
        });
        let event = inbox.poll()?;
        let mut finished = None;
        let (text, error, ended) = match event {
            Event::Receiving { name, bytes } => (
                format!("Receiving {name}... {:.0} MB", bytes as f64 / 1e6),
                false,
                false,
            ),
            Event::Installing { name } => (format!("Importing {name}..."), false, false),
            Event::Done(installed) => {
                log::info!("Horizon: import: {}", installed.message);
                finished = Some(installed.kind);
                if installed.kind == Kind::Skin {
                    self.skin = installed.path.clone();
                }
                (installed.message, false, true)
            }
            Event::Failed { name, error } => {
                log::warn!("Horizon: import of {name} failed: {error}");
                (error, true, true)
            }
        };
        self.note = Some(Note {
            text,
            error,
            ended: ended.then(Instant::now),
        });
        finished
    }

    /// Replace the note (an import that finished, told in other words by the caller).
    pub(super) fn say(&mut self, text: String) {
        self.note = Some(Note {
            text,
            error: false,
            ended: Some(Instant::now()),
        });
    }

    /// The note at the top of the screen, while an import runs and for a while after.
    pub(super) fn draw(&self, atlas: &HudAtlas, batch: &mut HudBatch) {
        let Some(note) = &self.note else { return };
        if note.ended.is_some_and(|t| t.elapsed() > SHOWN) {
            return;
        }
        let mut ui = Ui::new(atlas, batch, (-1.0, -1.0));
        let size = 13.0;
        let width = ui.measure(Face::Regular, size, &note.text).min(ui.w - 32.0);
        let (pad, height) = (12.0, 30.0);
        let x = (ui.w - width) / 2.0 - pad;
        ui.rounded(
            x,
            10.0,
            width + 2.0 * pad,
            height,
            8.0,
            color::with_alpha(color::BG, 0.9),
        );
        let ink = if note.error {
            color::DANGER
        } else if note.ended.is_some() {
            color::SUCCESS
        } else {
            color::TEXT
        };
        ui.text(
            Face::Regular,
            size,
            x + pad,
            10.0 + (height - size) / 2.0 - 2.0,
            ink,
            &note.text,
        );
        ui.finish();
    }
}

impl super::Game<'_> {
    /// The inbox, a few times a second, from the frame.
    pub(super) fn poll_imports(&mut self) {
        match self.imports.poll(&self.ui.vanilla_root.clone()) {
            Some(Kind::Resources) => self
                .imports
                .say("Minecraft resources imported: restart the game to use them".into()),
            Some(Kind::ResourcePack) => self.refresh_pack_lists(),
            // The Mods page lists it at once; a mod with code runs as soon as it is turned on.
            Some(Kind::Mod) => self.start_mods(),
            // A skin is worn at once (Skin Customization's Choose a File...).
            Some(Kind::Skin) => {
                if let Some(path) = self.imports.skin.take()
                    && !self.button_image_imported(&path)
                {
                    self.own_skin_imported(&path);
                }
            }
            _ => {}
        }
    }

    pub(super) fn draw_imports(&self, batch: &mut HudBatch) {
        self.imports.draw(&self.gui.atlas, batch);
    }
}
