//! Thin native view over the shared account and wardrobe APIs.
use super::{
    login::Login,
    pages::{Action, button},
};
use crate::{
    account::{Accounts, Provider},
    skins::{
        self, Model, cloud,
        library::{Kind, Library},
        preview,
    },
};
use horizon_client::{
    hud::rgba,
    ui_theme::{Face, Ui, color},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

pub(super) enum CloudAction {
    Upload,
    Cape(usize),
}
pub(super) struct Wardrobe {
    pub root: PathBuf,
    default_skin: PathBuf,
    library: Library,
    key: String,
    provider: Option<Provider>,
    pub capes: bool,
    pub camera: preview::Camera,
    cache: cloud::Cache,
    pub view: [f32; 3],
    worker: Option<mpsc::Receiver<(String, Result<(cloud::Cache, bool), String>)>>,
    bitmap: Vec<([f32; 4], u32)>,
    rect: [f32; 4],
    fingers: std::collections::BTreeMap<u64, [f32; 2]>,
    note: String,
    damaged: bool,
}
impl Wardrobe {
    pub fn new(home: &Path) -> Self {
        let root = home.join("wardrobe");
        let loaded = Library::load(&root);
        let damaged = loaded.is_err();
        let note = loaded.as_ref().err().cloned().unwrap_or_default();
        let mut value = Self {
            root,
            default_skin: home
                .join(".local/horizon-client")
                .join(horizon_core::VANILLA_VERSION)
                .join("assets/minecraft/textures/entity/player/wide/steve.png"),
            library: loaded.unwrap_or_default(),
            key: String::new(),
            provider: None,
            capes: false,
            camera: Default::default(),
            cache: Default::default(),
            worker: None,
            bitmap: vec![],
            rect: [0.0; 4],
            view: [0.0; 3],
            fingers: Default::default(),
            note,
            damaged,
        };
        value.preview();
        value
    }
    pub fn open(&mut self, offline: &str, client_id: &str) {
        let account = Accounts::load().selected().cloned();
        let key = account
            .as_ref()
            .map(|a| a.key())
            .unwrap_or_else(|| format!("offline:{}", offline.to_ascii_lowercase()));
        if self.key != key {
            // A previous provider's in-flight result must not delay this profile's cloud load.
            self.worker = None;
            self.fingers.clear();
            self.key = key;
            self.provider = account.map(|a| a.provider);
            self.cache = Default::default();
        }
        self.reload();
        if self.provider.is_some() && self.worker.is_none() {
            self.cloud(client_id, None);
        }
        self.preview();
    }
    pub fn reload(&mut self) {
        match Library::load(&self.root) {
            Ok(value) => {
                self.library = value;
                self.damaged = false;
                self.preview();
            }
            Err(error) => {
                self.damaged = true;
                self.note = error;
            }
        }
    }
    pub fn poll(&mut self) {
        if let Some((key, result)) = self.worker.as_ref().and_then(|r| r.try_recv().ok()) {
            self.worker = None;
            if key != self.key {
                return;
            }
            match result {
                Ok((cache, activated_cape)) => {
                    let selection = self.library.selections.entry(self.key.clone()).or_default();
                    if selection.skin.is_none() {
                        selection.model = if cache.slim {
                            Model::Slim
                        } else {
                            Model::Classic
                        };
                    }
                    if activated_cape {
                        selection.cape = None;
                        selection.hide_cape = false;
                        if let Err(error) = self.library.save(&self.root) {
                            self.note = error;
                            self.preview();
                            return;
                        }
                    }
                    self.cache = cache;
                    self.note =
                        "Cloud profile synchronized. Local overrides are kept separate.".into();
                    self.preview();
                }
                Err(error) => self.note = error,
            }
        }
    }
    fn kind(&self) -> Kind {
        if self.capes { Kind::Cape } else { Kind::Skin }
    }
    fn save(&mut self) {
        if let Err(error) = self.library.save(&self.root) {
            self.note = error;
        }
        self.preview();
    }
    pub fn select(&mut self, index: usize) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        if let Some(entry) = self.library.entries.get(index).cloned() {
            self.note = match self
                .library
                .select(&self.root, &self.key, &entry.id, entry.kind)
            {
                Ok(()) => "Applied locally. The account's cloud profile is unchanged.".into(),
                Err(error) => error,
            };
            self.preview();
        }
    }
    pub fn import(&mut self) {
        if self.damaged {
            return;
        }
        self.note = match crate::documents::begin_appearance(self.root.clone(), self.kind()) {
            Ok(()) => "Choose a PNG image in Android Files.".into(),
            Err(error) => error,
        };
    }
    pub fn use_cloud(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        if self.capes {
            selection.cape = None;
            selection.hide_cape = false;
        } else {
            selection.skin = None;
        }
        self.note = if self.provider.is_some() {
            "Using the account's cloud appearance. No cloud changes were made."
        } else {
            "Local override cleared. Using the default appearance."
        }
        .into();
        self.save();
    }
    pub fn model(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        selection.model = if selection.model == Model::Classic {
            Model::Slim
        } else {
            Model::Classic
        };
        self.save();
    }
    pub fn hide_cape(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        selection.hide_cape = !selection.hide_cape;
        self.save();
    }
    pub fn cloud(&mut self, client_id: &str, action: Option<CloudAction>) {
        if self.worker.is_some() {
            return;
        }
        let Some(provider) = self.provider else {
            self.note = "Offline appearance is local to this installation.".into();
            return;
        };
        if provider == Provider::Ely && action.is_some() {
            self.note =
                "Ely.by cloud skins are managed at ely.by. Custom capes can be applied locally."
                    .into();
            if let Err(error) = crate::platform::open_login_url("https://ely.by/skins") {
                self.note = error;
            }
            return;
        }
        let key = self.key.clone();
        let root = self.root.clone();
        let id = client_id.to_owned();
        let selection = self
            .library
            .selections
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let cape = match &action {
            Some(CloudAction::Cape(index)) => {
                self.cache.snapshot.capes.get(*index).map(|c| c.id.clone())
            }
            _ => None,
        };
        if matches!(action, Some(CloudAction::Upload)) && selection.skin.is_none() {
            self.note = "Select a local skin before uploading to your Microsoft profile.".into();
            return;
        }
        if matches!(action, Some(CloudAction::Cape(_))) && cape.is_none() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.worker = Some(receiver);
        self.note = "Contacting account service...".into();
        std::thread::spawn(move || {
            let result = (|| {
                let (account, secrets) = Login::session(&id)?
                    .ok_or("Sign in before synchronizing your cloud appearance.")?;
                if account.key() != key {
                    return Err("The selected account changed. Open the gallery again.".into());
                }
                match action {
                    Some(CloudAction::Upload) => {
                        let path = Library::image(
                            &root,
                            selection.skin.as_deref().ok_or("Select a skin first.")?,
                        )?;
                        cloud::upload_skin(
                            &account,
                            &secrets,
                            &skins::read_file(&path)?,
                            selection.model,
                        )?;
                    }
                    Some(CloudAction::Cape(_)) => {
                        cloud::activate_cape(&account, &secrets, cape.as_deref())?
                    }
                    None => {}
                }
                cloud::fetch(&account, &secrets, &root)
                    .map(|cache| (cache, matches!(action, Some(CloudAction::Cape(_)))))
            })();
            let _ = sender.send((key, result));
        });
    }
    pub fn prepare_game(&mut self, offline: &str, game: &Path) -> Result<(), String> {
        let key = Accounts::load()
            .selected()
            .map(|a| a.key())
            .unwrap_or_else(|| format!("offline:{}", offline.to_ascii_lowercase()));
        let cloud = if key == self.key {
            self.cache
                .skin
                .as_deref()
                .map(|p| (p, self.cache.slim, self.cache.cape.as_deref()))
        } else {
            None
        };
        Library::load(&self.root)?.prepare_game(&self.root, &key, game, cloud)
    }
    pub fn preview(&mut self) {
        let selection = self
            .library
            .selections
            .get(&self.key)
            .cloned()
            .unwrap_or_default();
        let path = |id: &Option<String>| {
            id.as_deref()
                .and_then(|id| Library::image(&self.root, id).ok())
        };
        let skin = path(&selection.skin)
            .or_else(|| self.cache.skin.clone())
            .and_then(|p| skins::read_file(&p).ok())
            .and_then(|b| skins::read_skin(&b).ok())
            .or_else(|| {
                let bytes = skins::read_file(&self.default_skin).ok()?;
                skins::read_skin(&bytes).ok()
            })
            .unwrap_or_else(preview::mannequin);
        let cape = if selection.hide_cape {
            None
        } else {
            path(&selection.cape)
                .or_else(|| self.cache.cape.clone())
                .and_then(|p| skins::read_file(&p).ok())
                .and_then(|b| skins::read_cape(&b).ok())
        };
        let model = selection.model;
        let image = preview::render(&skin, cape.as_ref(), model, self.camera, [160, 200]);
        self.bitmap.clear();
        for y in 0..image.height() {
            let mut x = 0;
            while x < image.width() {
                let color = image.get_pixel(x, y).0;
                let start = x;
                x += 1;
                while x < image.width() && image.get_pixel(x, y).0 == color {
                    x += 1;
                }
                if color[3] > 0 {
                    self.bitmap.push((
                        [start as f32, y as f32, (x - start) as f32, 1.0],
                        rgba(color[0], color[1], color[2], color[3]),
                    ));
                }
            }
        }
    }
    pub fn gesture(&mut self, touch: &winit::event::Touch, density: f32) -> bool {
        use winit::event::TouchPhase;
        let point = [
            touch.location.x as f32 / density,
            touch.location.y as f32 / density,
        ];
        if touch.phase == TouchPhase::Started {
            let [x, y, w, h] = self.rect;
            if point[0] >= x && point[0] < x + w && point[1] >= y && point[1] < y + h {
                self.fingers.insert(touch.id, point);
                return true;
            }
            return false;
        }
        let Some(old) = self.fingers.get(&touch.id).copied() else {
            return false;
        };
        if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.fingers.remove(&touch.id);
            return true;
        }
        if let Some(other) = self
            .fingers
            .iter()
            .find_map(|(id, p)| (*id != touch.id).then_some(*p))
        {
            let distance =
                |p: [f32; 2]| ((p[0] - other[0]).powi(2) + (p[1] - other[1]).powi(2)).sqrt();
            let before = distance(old);
            if before > 1.0 {
                self.camera.zoom(distance(point) / before);
            }
        } else {
            self.camera
                .rotate((point[0] - old[0]) * 0.7, (point[1] - old[1]) * 0.5);
        }
        self.fingers.insert(touch.id, point);
        self.preview();
        true
    }
    pub fn draw(&mut self, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>, scroll: f32) {
        let width = ui.w;
        let compact = width < 700.0;
        let top = 160.0;
        ui.text(Face::Medium, 26.0, 32.0, top, color::TEXT, "Skins & Capes");
        ui.text(
            Face::Regular,
            13.0,
            32.0,
            top + 36.0,
            color::MUTED,
            "Local choices change your view. Cloud buttons update your account.",
        );
        let preview_width = if compact {
            (width - 64.0).min(260.0)
        } else {
            300.0
        };
        let preview_height = if compact {
            (ui.h - 560.0).clamp(100.0, 190.0)
        } else {
            300.0
        };
        let px = if compact {
            (width - preview_width) * 0.5
        } else {
            32.0
        };
        let py = top + 66.0;
        self.rect = [px, py, preview_width, preview_height];
        ui.rounded(px, py, preview_width, preview_height, 4.0, color::RAISED);
        let scale = (preview_width / 160.0).min(preview_height / 200.0);
        let ox = px + (preview_width - 160.0 * scale) * 0.5;
        let oy = py + (preview_height - 200.0 * scale) * 0.5;
        for ([x, y, w, h], color) in &self.bitmap {
            ui.rounded(
                ox + x * scale,
                oy + y * scale,
                w * scale,
                h * scale,
                0.0,
                *color,
            );
        }
        let controls = py + preview_height + 6.0;
        for (i, (label, action)) in [
            ("<", Action::WardrobeRotate(-1)),
            (">", Action::WardrobeRotate(1)),
            ("-", Action::WardrobeZoom(false)),
            ("+", Action::WardrobeZoom(true)),
        ]
        .into_iter()
        .enumerate()
        {
            button(
                ui,
                targets,
                [px + i as f32 * 44.0, controls, 38.0, 30.0],
                label,
                action,
                false,
            );
        }
        let model = self
            .library
            .selections
            .get(&self.key)
            .map(|s| s.model)
            .unwrap_or_default();
        button(
            ui,
            targets,
            [px, controls + 38.0, preview_width, 32.0],
            if model == Model::Slim {
                "Alex / Slim"
            } else {
                "Steve / Classic"
            },
            Action::WardrobeModel,
            false,
        );
        let lx = if compact { 32.0 } else { 370.0 };
        let lw = if compact {
            width - 64.0
        } else {
            width - lx - 32.0
        };
        let listtop = if compact { controls + 82.0 } else { top + 66.0 };
        button(
            ui,
            targets,
            [lx, listtop, (lw - 8.0) * 0.5, 32.0],
            "Skins",
            Action::WardrobeKind(false),
            !self.capes,
        );
        button(
            ui,
            targets,
            [lx + (lw + 8.0) * 0.5, listtop, (lw - 8.0) * 0.5, 32.0],
            "Capes",
            Action::WardrobeKind(true),
            self.capes,
        );
        button(
            ui,
            targets,
            [lx, listtop + 40.0, lw, 34.0],
            "Import PNG to local gallery",
            Action::WardrobeImport,
            true,
        );
        let mut row = listtop + 84.0 - scroll;
        let bottom = ui.h - 80.0;
        let selection = self
            .library
            .selections
            .get(&self.key)
            .cloned()
            .unwrap_or_default();
        let selected = if self.capes {
            selection.cape.as_deref()
        } else {
            selection.skin.as_deref()
        };
        if self.capes && self.provider.is_some() {
            for (index, cape) in self.cache.snapshot.capes.iter().enumerate() {
                if row >= listtop + 80.0 && row + 32.0 <= bottom {
                    let label = ui.fit_text(
                        Face::Medium,
                        14.0,
                        &format!(
                            "{} · cloud{}",
                            cape.title,
                            if cape.active {
                                " · active"
                            } else {
                                " · activate"
                            }
                        ),
                        lw - 24.0,
                    );
                    button(
                        ui,
                        targets,
                        [lx, row, lw, 32.0],
                        &label,
                        Action::WardrobeActivateCape(index),
                        cape.active,
                    );
                }
                row += 38.0;
            }
        }
        for (index, entry) in self
            .library
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == self.kind())
        {
            if row >= listtop + 80.0 && row + 32.0 <= bottom {
                let label = ui.fit_text(
                    Face::Medium,
                    14.0,
                    &format!(
                        "{}  · local{}",
                        entry.title,
                        if selected == Some(&entry.id) {
                            " · selected"
                        } else {
                            ""
                        }
                    ),
                    lw - 24.0,
                );
                button(
                    ui,
                    targets,
                    [lx, row, lw, 32.0],
                    &label,
                    Action::WardrobeSelect(index),
                    selected == Some(&entry.id),
                );
            }
            row += 38.0;
        }
        if self.provider.is_none() {
            if row >= listtop + 80.0 && row + 32.0 <= bottom {
                button(
                    ui,
                    targets,
                    [lx, row, lw, 32.0],
                    "Reset to default locally",
                    Action::WardrobeUseCloud,
                    false,
                );
            }
            row += 38.0;
        }
        if self.provider.is_some() {
            for (label, action) in [
                ("Use current cloud appearance", Action::WardrobeUseCloud),
                ("Refresh cloud profile", Action::WardrobeCloud),
            ] {
                if row >= listtop + 80.0 && row + 32.0 <= bottom {
                    button(ui, targets, [lx, row, lw, 32.0], label, action, false);
                }
                row += 38.0;
            }
            if !self.capes {
                if row >= listtop + 80.0 && row + 32.0 <= bottom {
                    button(
                        ui,
                        targets,
                        [lx, row, lw, 32.0],
                        if self.provider == Some(Provider::Microsoft) {
                            "Upload selected skin to Microsoft"
                        } else {
                            "Manage Ely.by cloud skin on website"
                        },
                        Action::WardrobeUpload,
                        false,
                    );
                }
                row += 38.0;
            } else {
            }
        }
        if self.capes && row >= listtop + 80.0 && row + 32.0 <= bottom {
            button(
                ui,
                targets,
                [lx, row, lw, 32.0],
                "Hide / show cape locally",
                Action::WardrobeHideCape,
                false,
            );
        }
        self.view = [
            listtop + 80.0,
            bottom,
            (row + scroll + 38.0 - bottom).max(0.0),
        ];
        let note = ui.fit_text(Face::Regular, 12.0, &self.note, width - 64.0);
        ui.text(Face::Regular, 12.0, 32.0, ui.h - 60.0, color::MUTED, &note);
    }
}
