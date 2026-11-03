//! The player's own name and skin from Options > Skin Customization (horizon_client::own_skin):
//! the name kept for the next world or server, and the chosen PNG of `skins/` put in one of the
//! atlas's skin slots, where the local player's model (first and third person, the inventory's)
//! and the player list's head take it in place of the profile's skin (`update_remote_skins`).
use super::Game;
use horizon_client::{
    hud::HudBatch,
    mobs::{Mob, MobData, MobType},
    offline_name,
    options_ui::Page,
    own_skin::{self, SkinChoice},
    text::Text,
    title::Mode,
    ui,
};
use horizon_core::geometry::Vec3;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    mpsc::{Receiver, channel},
};
use std::time::{Duration, Instant};

/// The own skin's name in the atlas: one skin slot, written again when the skin changes.
const LOCATION: &str = "horizon/own_skin";

/// How far the title screen's player is turned toward the menu, in degrees, as Bedrock's stands.
const TITLE_TURN: f32 = 25.0;
/// Degrees the title screen's player turns per GUI pixel dragged.
const TITLE_TURN_PER_PIXEL: f32 = 2.0;

/// The chosen skin and its image, read on first use.
#[derive(Default)]
pub(super) struct OwnSkin {
    loaded: bool,
    choice: SkinChoice,
    /// The chosen skin's 64x64 RGBA; None for the default one.
    image: Option<Vec<u8>>,
    /// Whether the slot holds `image` as it is now.
    uploaded: bool,
    /// The title screen's player, its skin and name looked at every frame.
    title: Option<TitlePlayer>,
    /// Before the world's resources: the player's type read on a thread of its own, and the mob
    /// data its model is drawn with.
    title_model: Option<Receiver<Option<Arc<MobType>>>>,
    title_data: Option<MobData>,
    /// When the title screen first had its player to show (a skin to come is waited for a while),
    /// and whether it shows now (its drag and button answer only then).
    title_since: Option<Instant>,
    title_visible: bool,
    /// How far a drag turned it, and the drag going on: where it began and the turn then.
    title_turn: f32,
    title_drag: Option<(f32, f32)>,
    /// Ely.by's skins by name, kept in `assets/skins/elyby.json` once read, and whether Skin
    /// Customization has them on (read on first use).
    elyby: horizon_client::elyby::Lookup,
    elyby_kept: bool,
    elyby_on: Option<bool>,
    /// The name worlds open with, read again every few seconds.
    worlds_name: Option<(String, Instant)>,
}

/// The title screen's player: the model wearing the own skin, and the name worlds open with.
pub(super) struct TitlePlayer {
    mob: Mob,
    name: String,
}

/// What the player wears where no server gives a skin (`worn`).
enum Worn {
    Skin(String, bool),
    Default,
    /// Ely.by's answer or its skin's download, still to come.
    Waiting,
}

/// How long the title screen waits for a skin to come before it shows the default one.
const SKIN_WAIT: Duration = Duration::from_secs(3);

/// Where the title screen's player stands (GUI pixels).
struct TitleLayout {
    /// The model's box: the feet at its bottom, the hat inside it however it is turned.
    model: [f32; 4],
    /// GUI pixels a block.
    size: f32,
    /// Its Customize... button: x, y, width.
    button: [f32; 3],
}

const BUTTON_WIDTH: f32 = 90.0;

/// The title screen's player right of the menu's buttons (title.rs, Mode::Main: 200 wide in the
/// middle), a third of the screen tall and standing about where the scene's ground is, as
/// Bedrock's; its button under the feet. None where the screen leaves it too little room.
fn title_layout_in(w: f32, h: f32) -> Option<TitleLayout> {
    let (w, h) = (w.ceil(), h.ceil());
    let left = (w / 2.0).floor() + 100.0;
    let room = w - left;
    if room < BUTTON_WIDTH + 8.0 {
        return None;
    }
    let size = (h * 0.19).min(room / 1.6).min(72.0).floor();
    let mid = ((left + w) / 2.0).floor();
    let feet = (h * 0.74).floor();
    Some(TitleLayout {
        // `mob_preview` stands the feet 0.96 blocks under the box's middle.
        model: [
            mid - 0.7 * size,
            feet - 2.0 * size,
            mid + 0.7 * size,
            feet + 0.1 * size,
        ],
        size,
        button: [mid - BUTTON_WIDTH / 2.0, feet + 6.0, BUTTON_WIDTH],
    })
}

impl Game<'_> {
    /// Where horizon-player.txt is: beside options.txt.
    fn player_dir() -> PathBuf {
        Self::options_path()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    /// `skins/` beside options.txt.
    pub(super) fn skins_dir() -> PathBuf {
        Self::options_path().with_file_name(own_skin::DIR)
    }

    /// Wear `choice`: its image read now; why not, for a file that is no skin (the default skin
    /// shows then).
    fn put_on_skin(&mut self, choice: SkinChoice) -> Result<(), String> {
        let skin = &mut self.own_skin;
        skin.loaded = true;
        skin.image = None;
        skin.uploaded = false;
        skin.choice = choice;
        let Some(file) = skin.choice.file.clone() else {
            return Ok(());
        };
        skin.image = Some(own_skin::load(&Self::skins_dir(), &file)?);
        Ok(())
    }

    /// The local player's own skin in the atlas, and whether its model is slim; None for the
    /// profile's skin (the account's, else the default one).
    pub(super) fn own_skin_sprite(&mut self) -> Option<(String, bool)> {
        if !self.own_skin.loaded
            && let Err(e) = self.put_on_skin(SkinChoice::load(&Self::player_dir()))
        {
            log::warn!("Horizon: own skin: {e}");
        }
        // None for the default skin.
        self.own_skin.image.as_ref()?;
        // In its slot, marked shown; else (changed, evicted, or the atlas made again with the
        // packs) written there again.
        if !self.own_skin.uploaded
            || !self.gui.atlas.sprites.contains_key(LOCATION)
            || !self.show_skin(LOCATION)
        {
            let rgba = self.own_skin.image.clone()?;
            self.store_skin(LOCATION.to_owned(), rgba);
            self.own_skin.uploaded = true;
        }
        Some((LOCATION.to_owned(), self.own_skin.choice.slim))
    }

    /// Skin Customization's name: kept for the next world or server.
    pub(super) fn keep_player_name(&mut self, name: &str) {
        if let Err(e) = offline_name::choose(&Self::player_dir(), name) {
            log::warn!("Horizon: {e}");
            self.ui.options.skin_warning(e);
        }
        self.own_skin.worlds_name = None;
        self.own_skin.title = None;
    }

    /// Skin Customization's Ely.by Skins: kept, and the skins shown again.
    pub(super) fn keep_elyby(&mut self, on: bool) {
        if let Err(e) = horizon_client::elyby::set_enabled(&Self::player_dir(), on) {
            log::warn!("Horizon: could not keep Ely.by Skins: {e}");
        }
        self.own_skin.elyby_on = Some(on);
        self.own_skin.title = None;
    }

    fn elyby_enabled(&mut self) -> bool {
        if !self.own_skin.elyby_kept {
            self.own_skin.elyby_kept = true;
            let file = Self::player_dir().join("assets/skins/elyby.json");
            self.own_skin.elyby.keep_in(file);
        }
        *self
            .own_skin
            .elyby_on
            .get_or_insert_with(|| horizon_client::elyby::enabled(&Self::player_dir()))
    }

    /// `name`'s skin on Ely.by, where Skin Customization has Ely.by Skins on and it answered.
    pub(super) fn elyby_textures(&mut self, name: &str) -> Option<horizon_client::skins::Textures> {
        if !self.elyby_enabled() {
            return None;
        }
        self.own_skin.elyby.textures(name)
    }

    /// The name worlds open with (`offline_name::for_worlds`), read at most every few seconds.
    fn worlds_name(&mut self) -> String {
        if let Some((name, read)) = &self.own_skin.worlds_name
            && read.elapsed() < Duration::from_secs(5)
        {
            return name.clone();
        }
        let name = offline_name::for_worlds(&Self::player_dir());
        self.own_skin.worlds_name = Some((name.clone(), Instant::now()));
        name
    }

    /// The skin the player wears where no server gives one (single player, the title screen):
    /// the file Skin Customization chose, else, playing offline, the one the name worlds open
    /// with has on Ely.by. None for the default skin.
    pub(super) fn worn_skin(&mut self) -> Option<(String, bool)> {
        match self.own_worn() {
            Worn::Skin(skin, slim) => Some((skin, slim)),
            Worn::Default | Worn::Waiting => None,
        }
    }

    /// `worn_skin`, telling a skin still to come from the default one.
    fn own_worn(&mut self) -> Worn {
        use horizon_client::skins::{Download, Kind};
        if let Some((skin, slim)) = self.own_skin_sprite() {
            return Worn::Skin(skin, slim);
        }
        if horizon_client::auth::current().is_some() || !self.elyby_enabled() {
            return Worn::Default;
        }
        let name = self.worlds_name();
        let Some(textures) = self.own_skin.elyby.textures(&name) else {
            return if self.own_skin.elyby.waiting(&name) {
                Worn::Waiting
            } else {
                Worn::Default
            };
        };
        let Some(url) = textures.skin.clone() else {
            return Worn::Default;
        };
        match self.skin_downloads.borrow_mut().state(Kind::Skin, &url) {
            Download::Pending => return Worn::Waiting,
            Download::Failed => return Worn::Default,
            Download::Ready => {}
        }
        let location = {
            let mut downloads = self.skin_downloads.borrow_mut();
            horizon_client::skins::lookup(&textures, false, |kind, url| downloads.state(kind, url))
        };
        // The title screen has nothing else to put a download in the atlas: without this the skin
        // only came once a world had stored it.
        self.store_ready_skins();
        let Some(location) = location else {
            return Worn::Default;
        };
        // In its atlas slot; if not, queued again (the slot went to another skin) for the next frame.
        if self.again_skin(&location) {
            Worn::Skin(location, textures.slim)
        } else {
            Worn::Waiting
        }
    }

    /// The skins downloaded since the last call, put in the atlas's skin slots.
    pub(super) fn store_ready_skins(&mut self) {
        let ready: Vec<_> = self.skin_downloads.borrow_mut().take_ready();
        for (location, rgba) in ready {
            self.store_skin(location, rgba);
        }
    }

    /// Skin Customization's skin or model: kept and put on.
    pub(super) fn choose_own_skin(&mut self, choice: SkinChoice) {
        if let Err(e) = choice.save(&Self::player_dir()) {
            log::warn!("Horizon: could not keep the skin: {e}");
            self.ui
                .options
                .skin_warning(format!("Could not keep the skin: {e}"));
        }
        if let Err(e) = self.put_on_skin(choice) {
            log::warn!("Horizon: own skin: {e}");
            self.ui.options.skin_warning(e);
        }
    }

    /// Skin Customization's Choose a File...: the system's picker, whose file comes through the
    /// import inbox (`own_skin_imported`).
    pub(super) fn pick_skin_file(&mut self) {
        if !horizon_client::platform::pick_file() {
            self.ui
                .options
                .skin_warning("No file picker opened: put the skin in the skins folder".into());
        }
    }

    /// A skin the import inbox put in `skins/`: worn at once, the page showing it.
    pub(super) fn own_skin_imported(&mut self, path: &Path) {
        let Some(file) = path.file_name().and_then(|f| f.to_str()) else {
            return;
        };
        if !self.own_skin.loaded {
            let _ = self.put_on_skin(SkinChoice::load(&Self::player_dir()));
        }
        let choice = SkinChoice {
            file: Some(file.to_owned()),
            slim: self.own_skin.choice.slim,
        };
        if let Err(e) = choice.save(&Self::player_dir()) {
            log::warn!("Horizon: could not keep the skin: {e}");
        }
        let worn = self.put_on_skin(choice);
        self.ui.options.refresh_skin_page();
        if let Err(e) = worn {
            log::warn!("Horizon: own skin: {e}");
            self.ui.options.skin_warning(e);
        }
    }

    /// Whether the title screen is on its main page, where its player stands.
    fn title_main(&self) -> bool {
        self.title
            .as_ref()
            .is_some_and(|t| matches!(t.mode, Mode::Main))
    }

    /// Where the title screen shows its player in a GUI `w` by `h`, once it is drawn there; None
    /// before, and on the title's other pages.
    fn title_layout(&self, w: f32, h: f32) -> Option<TitleLayout> {
        (self.title_main() && self.own_skin.title_visible)
            .then(|| title_layout_in(w, h))
            .flatten()
    }

    /// The title screen's player's layout in the window as it is now.
    fn title_layout_now(&self) -> Option<TitleLayout> {
        let r = self.renderer.as_ref()?;
        let b = HudBatch::new([r.config.width, r.config.height]);
        self.title_layout(b.width, b.height)
    }

    /// Whether `(x, y)` (GUI pixels) is on the title screen's player, which a drag turns.
    pub(super) fn on_title_player(&self, (x, y): (f32, f32)) -> bool {
        self.title_layout_now().is_some_and(|l| {
            let [x0, y0, x1, y1] = l.model;
            x >= x0 && x < x1 && y >= y0 && y < y1
        })
    }

    /// Whether `cursor` is on the title screen's player or its Customize... button.
    pub(super) fn title_player_at(&self, cursor: (f32, f32)) -> bool {
        self.on_title_player(cursor)
            || self.title_layout_now().is_some_and(|l| {
                let [x, y, w] = l.button;
                ui::hovered(cursor, x, y, w, 20.0)
            })
    }

    /// A press on the title screen's player: Customize... opens Skin Customization, the player
    /// itself only turns while dragged (a tap there does nothing, so a drag never opens it).
    pub(super) fn press_title_player(&mut self) {
        if self.on_title_player(self.cursor) {
            self.own_skin.title_drag = Some((self.cursor.0, self.own_skin.title_turn));
        } else if self.title_player_at(self.cursor) {
            self.audio.play_button_click();
            self.open_options_at(true, Page::Skin);
        }
    }

    /// The pointer moved: a drag on the title screen's player turns it, rightward to its left.
    pub(super) fn drag_title_player(&mut self) {
        if let Some((x, turn)) = self.own_skin.title_drag {
            self.own_skin.title_turn = turn - (self.cursor.0 - x) * TITLE_TURN_PER_PIXEL;
        }
    }

    /// The pointer let go: the drag ends where it turned the player.
    pub(super) fn release_title_player(&mut self) {
        self.own_skin.title_drag = None;
    }

    /// The title screen's player beside the menu, as Bedrock's: turned toward it and turning
    /// under a drag, the arms swaying as a still player's do, the name worlds open with over
    /// the head, and the button that changes skin and name under the feet.
    pub(super) fn draw_title_player(&mut self, batch: &mut HudBatch) {
        self.own_skin.title_visible = false;
        if !self.title_main() {
            return;
        }
        if self.own_skin.title.is_none() {
            self.read_title_player();
        }
        if self.own_skin.title.is_none() {
            return;
        }
        // The skin and the name every frame (both kept, cheap to ask): a skin waiting for its
        // atlas slot or its download shows the first frame it has one, and the default one only
        // when it takes long (the first time a name is asked of Ely.by).
        let worn = self.own_worn();
        let since = *self.own_skin.title_since.get_or_insert_with(Instant::now);
        if matches!(worn, Worn::Waiting) && since.elapsed() < SKIN_WAIT {
            return;
        }
        let name = self.worlds_name();
        let Some(player) = self.own_skin.title.as_mut() else {
            return;
        };
        (player.mob.skin, player.mob.slim) = match worn {
            Worn::Skin(skin, slim) => (Some(skin), slim),
            Worn::Default | Worn::Waiting => (None, false),
        };
        player.name = name;
        self.own_skin.title_visible = true;
        let Some(layout) = self.title_layout(batch.width, batch.height) else {
            return;
        };
        // The world's mob data once it loaded, else the title's own (only the model is drawn).
        let data = if self.res.rest_ready() {
            self.res.mob_data()
        } else if let Some(data) = &self.own_skin.title_data {
            data
        } else {
            return;
        };
        let Some(player) = &self.own_skin.title else {
            return;
        };
        let [x0, y0, x1, _] = layout.model;
        let mid = (x0 + x1) / 2.0;
        // AnimationUtils.bobModelPart reads the age; the head looks a little around, slowly.
        let seconds = self.started.elapsed().as_secs_f32();
        let mut mob = player.mob.clone();
        mob.age = (seconds * 20.0) as u32;
        let centre = (
            mid - 8.0 * (seconds * 0.45).sin(),
            (y0 + layout.model[3]) / 2.0,
        );
        let turn = TITLE_TURN + self.own_skin.title_turn;
        let (rect, size) = (layout.model, layout.size);
        self.mob_preview(&mob, data, batch, rect, size, 0.0625, centre, turn, false);
        let atlas = &self.gui.atlas;
        let name_x = (mid - HudBatch::text_width(atlas, &player.name) / 2.0).floor();
        batch.text(atlas, &player.name, name_x, y0 - 8.0, u32::MAX, true);
        let [bx, by, bw] = layout.button;
        let hover = ui::hovered(self.cursor, bx, by, bw, 20.0);
        let label = Text::literal("Customize...");
        ui::button(batch, atlas, bx, by, bw, 20.0, &label, true, hover);
    }

    /// The title screen's player's model (its skin and name come every frame): the world's
    /// player type once its resources loaded, else one read beside the menu, on a thread of its
    /// own (the entity models' file is large), with mob data of the blocks alone to draw it.
    fn read_title_player(&mut self) {
        let kind = if self.res.rest_ready() {
            self.res.player_type().clone()
        } else if let Some(receive) = &self.own_skin.title_model {
            receive.try_recv().ok().flatten()
        } else {
            let (send, receive) = channel();
            let root = self.options.resources.clone();
            let read = std::thread::Builder::new()
                .name("title player".into())
                .spawn(move || {
                    let _ = send.send(super::perspective::player_type(&root));
                });
            if let Err(e) = read {
                log::warn!("Horizon: the title screen's player: {e}");
            }
            self.own_skin.title_model = Some(receive);
            None
        };
        let Some(kind) = kind else {
            return;
        };
        if !self.res.rest_ready() {
            self.own_skin.title_data = Some(MobData::fallback(self.res.facts()));
        }
        self.own_skin.title_model = None;
        self.own_skin.title = Some(TitlePlayer {
            mob: Mob::new(0, kind, Vec3::ZERO, 0.0),
            name: String::new(),
        });
    }
}
