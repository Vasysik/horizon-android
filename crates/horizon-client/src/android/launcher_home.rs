//! The Android side of the glass launcher: reads the files and config into a [`View`], draws a
//! frame with its overlays, and does what Home's, Versions' and Settings' buttons ask.
use super::*;
use crate::catalog::Page;
use crate::launcher_data::{self as data, Instances, Pictures, Release};
use crate::launcher_glass::{self as glass, Kind, white};
use crate::launcher_view::{
    ACCENTS, Action, BundledRow, Form, Game, HUES, HomeTab, InstanceRow, LocalRow, Menu, MenuRow,
    Out, ProjectRow, ReleaseRow, ServerRow, SettingsTab, SettingsView, ShotRow, View, WorldRow,
    dialog, shot_viewer,
};
use horizon_client::{content_files::Kind as Content, osk::Output};
use std::{collections::BTreeSet, sync::Arc};

/// How long a note stays over the page.
const NOTE: Duration = Duration::from_secs(7);

/// What the open form is for.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum FormKind {
    NewInstance,
    CopyInstance,
    EditInstance,
    AddServer,
}

pub(super) struct Confirm {
    title: String,
    note: String,
    label: String,
    action: Action,
}

/// Home's lists and the overlays over the pages.
#[derive(Default)]
pub(super) struct Home {
    pub art: glass::Art,
    pub tab: HomeTab,
    pub settings_tab: SettingsTab,
    pub drawer: bool,
    pub instances: Instances,
    pub worlds: Vec<crate::worlds::World>,
    worlds_worker: Option<mpsc::Receiver<Vec<crate::worlds::World>>>,
    pub servers: Vec<crate::server_list::Server>,
    server_icons: Vec<Option<(u64, glass::Pixels)>>,
    pub shots: Vec<crate::screenshots::Shot>,
    pub pictures: Pictures,
    pub friend: String,
    menu: Option<Menu>,
    form: Option<(FormKind, String, String, usize)>,
    confirm: Option<Confirm>,
    viewer: Option<usize>,
    /// The world, server, data pack or shot a question is about.
    pending: usize,
    /// The world whose data packs a menu shows.
    packs: Vec<crate::datapacks::DataPack>,
    pack_world: usize,
    pub data_world: Option<usize>,
    pub releases: Vec<Release>,
    releases_note: String,
    releases_worker: Option<mpsc::Receiver<std::result::Result<Vec<Release>, String>>>,
    open_notes: BTreeSet<usize>,
    task: Option<mpsc::Receiver<std::result::Result<String, String>>>,
    /// Where the next Play goes, for one run.
    pub once_world: Option<String>,
    pub once_server: Option<String>,
    /// The run under way, for its play time.
    pub started: Option<(u32, Instant)>,
    /// The note shown, and since when.
    note: (String, Option<Instant>),
}

fn abi() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64-v8a",
        "x86_64" => "x86_64",
        "x86" => "x86",
        _ => "armeabi-v7a",
    }
}

fn links() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../launcher/links.json")).unwrap_or_default()
}

impl Launcher {
    /// The folder of the instance that plays.
    fn game(&self) -> PathBuf {
        self.catalog.config.game_home(&self.home)
    }

    /// Reads Home's lists again: on opening it, after a run, and after a change.
    pub(super) fn reload_home(&mut self) {
        let game = self.game();
        self.home_state.instances = Instances::load(&self.home);
        self.home_state.servers = crate::server_list::List::load(&game).servers();
        self.home_state.server_icons = self
            .home_state
            .servers
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let pixels = glass::decode_png(s.icon.as_deref()?, 64)?;
                Some((0x5e7_0000 + i as u64, Arc::new(pixels)))
            })
            .collect();
        self.home_state.shots = crate::screenshots::list(&game);
        let (sender, receiver) = mpsc::channel();
        self.home_state.worlds_worker = Some(receiver);
        // Each world's size walks its folder.
        std::thread::spawn(move || {
            let _ = sender.send(crate::worlds::list(&game));
        });
    }

    pub(super) fn load_releases(&mut self) {
        if self.home_state.releases_worker.is_some() {
            return;
        }
        let home = self.home.clone();
        let (sender, receiver) = mpsc::channel();
        self.home_state.releases_worker = Some(receiver);
        self.home_state.releases_note = "Reading the published builds…".into();
        std::thread::spawn(move || {
            let _ = sender.send(data::fetch_releases(&home));
        });
    }

    /// Runs `job` away from the frame; what it returns becomes the note over the page.
    fn task(
        &mut self,
        status: &str,
        job: impl FnOnce() -> std::result::Result<String, String> + Send + 'static,
    ) {
        if self.home_state.task.is_some() {
            return;
        }
        self.catalog.status = status.into();
        let (sender, receiver) = mpsc::channel();
        self.home_state.task = Some(receiver);
        std::thread::spawn(move || {
            let _ = sender.send(job());
        });
    }

    pub(super) fn poll_home(&mut self) {
        let h = &mut self.home_state;
        h.pictures.poll();
        if let Some(list) = h.worlds_worker.as_ref().and_then(|w| w.try_recv().ok()) {
            h.worlds_worker = None;
            h.worlds = list;
            h.data_world = h
                .data_world
                .filter(|i| *i < h.worlds.len())
                .or((!h.worlds.is_empty()).then_some(0));
        }
        if let Some(result) = h.releases_worker.as_ref().and_then(|w| w.try_recv().ok()) {
            h.releases_worker = None;
            match result {
                Ok(list) => {
                    h.releases_note = if list.is_empty() {
                        "No published builds.".into()
                    } else {
                        String::new()
                    };
                    h.releases = list;
                }
                Err(error) => h.releases_note = format!("Couldn't read the builds: {error}"),
            }
        }
        if let Some(result) = h.task.as_ref().and_then(|w| w.try_recv().ok()) {
            h.task = None;
            self.catalog.status = result.unwrap_or_else(|e| e);
            self.reload_home();
        }
        if let Some(kind) = self.catalog.show_home.take() {
            self.home_state.tab = match kind {
                Content::Mods => HomeTab::Mods,
                _ => HomeTab::Packs,
            };
            self.catalog.scroll = 0.0;
            self.reload_home();
        }
    }

    /// The note over the page, for a few seconds; an error stays.
    fn note(&mut self) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        let h = &mut self.home_state;
        if h.note.0 != self.catalog.status {
            h.note = (self.catalog.status.clone(), Some(Instant::now()));
        }
        let busy = self.catalog.busy() || h.task.is_some();
        if !busy && h.note.1.is_some_and(|since| since.elapsed() > NOTE) {
            return String::new();
        }
        h.note.0.clone()
    }

    fn view(&mut self) -> View {
        use horizon_client::official_install::InstallStage;
        let note = self.note();
        let installed = horizon_client::official_install::ready(&self.home);
        let progress = *self.install.lock().unwrap();
        let current = InstallStage::ALL
            .iter()
            .position(|s| *s == progress.stage)
            .unwrap_or(0);
        let fraction = if progress.total > 0 {
            progress.done as f32 / progress.total as f32
        } else {
            0.0
        };
        let game = Game {
            installed,
            installing: self.worker.is_some(),
            steps: InstallStage::ALL
                .iter()
                .enumerate()
                .map(|(i, s)| (s.label().to_owned(), i < current, i == current))
                .collect(),
            overall: (current as f32 + fraction.clamp(0.0, 1.0)) / InstallStage::ALL.len() as f32,
            detail: match progress.stage {
                InstallStage::Download => format!(
                    "{:.1} / {:.1} MiB, verified as they come. Finished files are kept.",
                    progress.done as f64 / 1048576.0,
                    progress.total as f64 / 1048576.0
                ),
                InstallStage::Controls => format!(
                    "{} / {} control files verified",
                    progress.done, progress.total
                ),
                _ => "Please keep Horizon open. Downloaded files are kept for a retry.".into(),
            },
            stage: progress.stage.label().to_owned(),
            failed: self.error.is_some(),
            preparing: self.session_worker.is_some(),
        };
        let c = &self.catalog.config;
        let h = &mut self.home_state;
        let selected = c.profile;
        let instances: Vec<_> = h
            .instances
            .list
            .iter()
            .map(|(id, m)| InstanceRow {
                id: *id,
                name: m.name.clone(),
                note: if *id == selected && !installed {
                    "Needs its files".into()
                } else if m.played_secs > 0 {
                    data::played(m.played_secs)
                } else {
                    format!("Ready · {}", horizon_core::VANILLA_VERSION)
                },
                hue: HUES[m.hue % HUES.len()],
                selected: *id == selected,
            })
            .collect();
        let played = data::played(
            h.instances
                .list
                .get(&selected)
                .map_or(0, |m| m.played_secs),
        );
        let worlds: Vec<_> = {
            let list = std::mem::take(&mut h.worlds);
            let rows = list
                .iter()
                .map(|w| WorldRow {
                    name: w.name.clone(),
                    note: format!(
                        "{} · {}{} · {}",
                        w.mode,
                        w.version.as_deref().map_or(String::new(), |v| format!("{v} · ")),
                        data::age(w.last_played),
                        data::size(w.size)
                    ),
                    icon: w.icon.as_ref().and_then(|p| h.pictures.get(p, 64)),
                })
                .collect();
            h.worlds = list;
            rows
        };
        let servers = h
            .servers
            .iter()
            .zip(h.server_icons.iter().chain(std::iter::repeat(&None)))
            .map(|(s, icon)| ServerRow {
                name: s.name.clone(),
                address: s.ip.clone(),
                icon: icon.clone(),
            })
            .collect();
        let shots = {
            let list = std::mem::take(&mut h.shots);
            let rows = list
                .iter()
                .map(|s| ShotRow {
                    name: s.name.clone(),
                    thumb: h.pictures.get(&s.path, 360),
                })
                .collect();
            h.shots = list;
            rows
        };
        let friend_valid = horizon_friends::invite::parse(&h.friend).is_some();
        let page = self.catalog.page;
        let query = self.catalog.query.to_lowercase();
        let on = &self.catalog.bundled_enabled;
        let bundled: Vec<_> = horizon_client::mod_list::ANDROID_BUNDLED
            .into_iter()
            .filter(|(id, title, _)| {
                page != Page::Mods
                    || c.mod_source == crate::shader_catalog::Origin::Horizon
                        && (id.contains(&query) || title.to_lowercase().contains(&query))
            })
            .map(|(id, title, note)| BundledRow {
                id,
                title: title.to_owned(),
                note: note.to_owned(),
                enabled: on.contains(id),
            })
            .collect();
        let local = self
            .catalog
            .local
            .iter()
            .enumerate()
            .map(|(index, l)| LocalRow {
                index,
                kind: l.kind,
                title: l.title.clone(),
                enabled: l.enabled,
            })
            .collect();
        let results = self
            .catalog
            .results
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                !(page == Page::Mods
                    && p.source == crate::browser::SourceId::Horizon
                    && horizon_client::mod_list::ANDROID_BUNDLED
                        .iter()
                        .any(|(id, _, _)| *id == p.id))
            })
            .map(|(index, p)| {
                let state = self.catalog.download_label(p).or_else(|| {
                    if p.kind == crate::browser::Kind::Mod
                        && p.source == crate::browser::SourceId::Modrinth
                    {
                        Some("Java mod".into())
                    } else if crate::catalog::source_only_mod(p) {
                        Some("Needs compiling".into())
                    } else if p.kind == crate::browser::Kind::Mod
                        && !cfg!(target_pointer_width = "64")
                    {
                        Some("64-bit only".into())
                    } else {
                        None
                    }
                });
                ProjectRow {
                    index,
                    title: p.title.clone(),
                    note: p.summary.clone(),
                    state,
                    install: Some(self.catalog.install_button(p).into()),
                }
            })
            .collect();
        let search_hint = match page {
            Page::Mods if c.mod_source == crate::shader_catalog::Origin::Modrinth => {
                "Modrinth's Java mods are shown to browse: Fabric, Forge and NeoForge can't run here."
            }
            Page::Shaders if c.shader_source == crate::shader_catalog::Origin::Modrinth => {
                "Shader ZIPs are experimental: some packs may not work."
            }
            Page::DataPacks => "From Modrinth. Only what Horizon reads is installed.",
            _ => "",
        };
        let source = match page {
            Page::Mods => Some(c.mod_source),
            Page::Shaders => Some(c.shader_source),
            _ => None,
        };
        let current_tag = format!("v{}", crate::backend::VERSION.trim_end_matches("-dev"));
        let releases = h
            .releases
            .iter()
            .enumerate()
            .map(|(i, r)| ReleaseRow {
                name: r.name.clone().unwrap_or_else(|| r.tag_name.clone()),
                note: format!(
                    "{}{}{}",
                    r.date(),
                    r.asset(abi())
                        .map_or(String::new(), |a| format!(" · {}", data::size(a.size))),
                    if r.prerelease { " · Pre-release" } else { "" }
                ),
                body: r.notes(),
                open: h.open_notes.contains(&i),
                current: r.tag_name == current_tag,
            })
            .collect();
        let accent = ACCENTS
            .iter()
            .position(|(_, v)| *v == c.accent_or_default())
            .unwrap_or(0);
        let settings = SettingsView {
            tab: h.settings_tab,
            accent,
            plain: c.plain_plates,
            still: c.still_scene,
            renderer: c.renderer.label().into(),
            resolution: if self.catalog.editing == Some("render_scale") {
                self.catalog.render_scale_text.clone()
            } else {
                format!("{}%", c.render_scale.percent())
            },
            workers: if c.worker_limit == 0 {
                "No additional limit".into()
            } else {
                c.worker_limit.to_string()
            },
            cache: if c.cache_budget_mib == 0 {
                "Engine default (80 MiB)".into()
            } else {
                format!("{} MiB", c.cache_budget_mib)
            },
            orientation: c.orientation.label().into(),
            input: c.input_host.label().into(),
            immersive: c.screen.immersive,
            cutout: c.screen.avoid_cutout,
            aspect: c.screen.aspect.label().into(),
            display: c.device.display_label(),
            hints: c.device.performance_hints,
            gpu_profile: c.device.gpu_profile,
            recording: c.record_performance,
            game_args: c.game_args.clone(),
            proxy: self.catalog.proxy_text.clone(),
            server: c.server.clone(),
            device: abi().into(),
        };
        View {
            version: crate::backend::VERSION.into(),
            minecraft: horizon_core::VANILLA_VERSION.into(),
            page,
            home_tab: h.tab,
            drawer: h.drawer,
            account_name: self.login.chip_name(&c.name),
            account_how: self.login.chip_how(),
            instances,
            played,
            game,
            status: note,
            editing: self.catalog.editing,
            friend: h.friend.clone(),
            friend_valid,
            bits: cfg!(target_pointer_width = "64"),
            worlds,
            servers,
            local,
            bundled,
            shots,
            log: data::log_lines(),
            follow: true,
            query: self.catalog.query.clone(),
            source,
            search_hint: search_hint.into(),
            results,
            offset: self.catalog.offset,
            data_world: h
                .data_world
                .and_then(|i| h.worlds.get(i))
                .map(|w| w.name.clone()),
            releases,
            releases_note: h.releases_note.clone(),
            settings,
        }
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        if self.renderer.as_ref().is_none_or(|r| r.suspended) || self.gui.is_none() {
            return Ok(());
        }
        let view = self.view();
        let (Some(r), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        let c = &self.catalog.config;
        let art = &mut self.home_state.art;
        art.plain = c.plain_plates;
        art.still = c.still_scene;
        art.prepare(ui.w, ui.h, c.accent_or_default());
        crate::launcher_glass::remember_look(art.look);
        // OEM navigation bars can remain visible even in fullscreen (keypad phones).
        // Inset the launcher layout only; keep surface and input coordinates intact.
        if let Some(window) = &self.window {
            use winit::platform::android::WindowExtAndroid;
            let rect = window.content_rect();
            let input = winit::platform::android::native_input_scale();
            if rect.bottom > 0 {
                ui.h = ui.h.min(rect.bottom as f32 * input[1] as f32 / ui.density);
            }
            if rect.right > 0 {
                ui.w = ui.w.min(rect.right as f32 * input[0] as f32 / ui.density);
            }
        }
        if let Some(bottom) = crate::native_ime::viewport_bottom() {
            ui.h = ui.h.min(bottom as f32 / ui.density);
        }
        let art = &self.home_state.art;
        art.background(&mut ui);
        let mut out = Out::default();
        view.draw(art, &mut ui, self.catalog.scroll, &mut out);
        if view.page == Page::Skins {
            let region = crate::launcher_view::area(&ui).rect;
            let start = ui.batch.vertices.len();
            let first = out.targets.len();
            let bottom = self
                .wardrobe
                .draw(art, &mut ui, &mut out.targets, self.catalog.scroll, region);
            view.finish_page(&mut ui, start, first, region, bottom, &mut out);
        }
        if self.home_state.drawer {
            view.drawer(art, &mut ui, &mut out);
        }
        if let Some(menu) = &self.home_state.menu {
            menu.draw(art, &mut ui, &mut out);
        }
        if let Some(form) = self.home_state.form_view() {
            form.draw(art, &mut ui, &mut out, self.catalog.editing);
        }
        if let Some(c) = &self.home_state.confirm {
            dialog(
                art,
                &mut ui,
                &mut out,
                &c.title,
                &c.note,
                &[
                    ("Cancel", Action::Dismiss, Kind::Secondary),
                    (&c.label, c.action, Kind::Danger),
                ],
            );
        }
        if let Some(i) = self.home_state.viewer
            && let Some(shot) = self.home_state.shots.get(i)
        {
            let image = self.home_state.pictures.get(&shot.path, 1024);
            shot_viewer(
                &self.home_state.art,
                &mut ui,
                &mut out,
                &shot.name,
                image.as_ref(),
            );
        }
        let art = &self.home_state.art;
        self.targets = std::mem::take(&mut out.targets);
        self.list_targets = std::mem::take(&mut out.list);
        self.list_view = out.scroll;
        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                art,
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing,
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(art, &mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        let w = ui.w;
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 7.0, y, 3.0, thumb, 1.5, white(0.35));
        }
        if self.exit_confirmation {
            let mut out = Out::default();
            dialog(
                art,
                &mut ui,
                &mut out,
                "Exit Horizon?",
                "",
                &[
                    ("Cancel", Action::Dismiss, Kind::Secondary),
                    ("Exit", Action::ConfirmExit, Kind::Primary),
                ],
            );
            self.targets = out.targets;
            self.list_targets.clear();
            self.list_view = None;
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            let ring = horizon_client::ui_theme::color::hex(art.look.accent_soft, 255);
            ui.border(*x - 2.0, *y - 2.0, *w + 4.0, *h + 4.0, 12.0, ring);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        if self.mouse_down || self.drag.as_ref().is_some_and(|drag| !drag.moved) {
            let point = [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ];
            if let Some(([x, y, w, h], _)) = self.targets.iter().rev().find(|(rect, _)| {
                point[0] >= rect[0]
                    && point[0] <= rect[0] + rect[2]
                    && point[1] >= rect[1]
                    && point[1] <= rect[1] + rect[3]
            }) && *w < ui.w
            {
                ui.rounded(*x, *y, *w, *h, 10.0, white(0.12));
            }
        }
        if !self.mouse_down
            && self.drag.is_none()
            && let Some((action, time)) = self.feedback
            && time.elapsed() < std::time::Duration::from_millis(160)
            && let Some(([x, y, w, h], _)) = self.targets.iter().find(|(_, a)| *a == action)
            && *w < ui.w
        {
            ui.rounded(*x, *y, *w, *h, 10.0, white(0.12));
        }
        crate::documents::draw_ui(&mut ui);
        ui.finish();

        if !r.render(&game::loading_frame(&b))? && !b.patches.is_empty() {
            // A suspended/lost surface skips the frame, including its glyph
            // uploads. Rebuild them next time instead of caching invisible text.
            horizon_client::ui_theme::reset_atlas();
        }
        Ok(())
    }

    /// What Back closes first; false when nothing is open.
    pub(super) fn close_overlay(&mut self) -> bool {
        let h = &mut self.home_state;
        if h.viewer.take().is_some() {
            return true;
        }
        if h.confirm.take().is_some() || h.menu.take().is_some() || h.form.take().is_some() {
            self.catalog.editing = None;
            crate::platform::hide();
            return true;
        }
        if std::mem::take(&mut h.drawer) {
            return true;
        }
        false
    }

    /// Typed text for a field of a form or of Home; false when it belongs to another page.
    pub(super) fn edit_home(&mut self, output: &Output) -> bool {
        let Some(name) = self.catalog.editing else {
            return false;
        };
        let h = &mut self.home_state;
        let value = match (name, &mut h.form) {
            ("friend", _) => &mut h.friend,
            ("instance_name" | "server_name", Some(form)) => &mut form.1,
            ("server_address", Some(form)) => &mut form.2,
            _ => return false,
        };
        match output {
            Output::Text(text) => value.push_str(text),
            Output::Backspace => {
                value.pop();
            }
            Output::Enter | Output::Close => {
                self.catalog.editing = None;
                crate::platform::hide();
            }
        }
        true
    }

    /// A field's text, when it's one of Home's.
    pub(super) fn home_field(&self, name: &str) -> Option<String> {
        let h = &self.home_state;
        match (name, &h.form) {
            ("friend", _) => Some(h.friend.clone()),
            ("instance_name" | "server_name", Some(form)) => Some(form.1.clone()),
            ("server_address", Some(form)) => Some(form.2.clone()),
            _ => None,
        }
    }

    fn select_instance(&mut self, id: u32) -> Result<()> {
        std::fs::create_dir_all(data::instance_home(&self.home, id))?;
        self.catalog.config.profile = id;
        self.catalog.config.save(&self.home)?;
        self.catalog.refresh(&self.home);
        self.home_state.drawer = false;
        self.session_ready = false;
        self.prepared_session = None;
        self.reload_home();
        Ok(())
    }

    fn world_menu(&mut self, i: usize) {
        let Some(world) = self.home_state.worlds.get(i) else {
            return;
        };
        self.home_state.menu = Some(Menu {
            title: world.name.clone(),
            note: format!("{} · {}", world.mode, data::size(world.size)),
            rows: vec![
                MenuRow {
                    icon: "play",
                    label: "Play".into(),
                    action: Action::PlayWorld(i),
                    danger: false,
                },
                MenuRow {
                    icon: "download-04",
                    label: "Back up".into(),
                    action: Action::BackupWorld(i),
                    danger: false,
                },
                MenuRow {
                    icon: "copy-01",
                    label: "Copy".into(),
                    action: Action::CopyWorld(i),
                    danger: false,
                },
                MenuRow {
                    icon: "package-01",
                    label: "Data packs".into(),
                    action: Action::WorldPacks(i),
                    danger: false,
                },
                MenuRow {
                    icon: "delete-02",
                    label: "Delete".into(),
                    action: Action::DeleteWorld(i),
                    danger: true,
                },
            ],
        });
    }

    fn pack_menu(&mut self, world: usize) {
        let h = &mut self.home_state;
        let Some(w) = h.worlds.get(world) else {
            return;
        };
        h.pack_world = world;
        h.packs = crate::datapacks::list(&w.path);
        let rows: Vec<_> = h
            .packs
            .iter()
            .enumerate()
            .map(|(i, p)| MenuRow {
                icon: "delete-02",
                label: format!("Remove {}", p.file),
                action: Action::RemoveDataPack(i),
                danger: true,
            })
            .chain(std::iter::once(MenuRow {
                icon: "search-01",
                label: "Find data packs".into(),
                action: Action::Page(Page::DataPacks),
                danger: false,
            }))
            .collect();
        h.data_world = Some(world);
        h.menu = Some(Menu {
            title: format!("{}: data packs", w.name),
            note: if h.packs.is_empty() {
                "None yet. The game turns a new one on when the world opens.".into()
            } else {
                format!("{} in this world.", h.packs.len())
            },
            rows,
        });
    }

    /// Does `action` if it's one of the glass launcher's own; false for the older ones.
    pub(super) fn home_action(
        &mut self,
        action: Action,
        events: &ActiveEventLoop,
    ) -> Result<bool> {
        let game = self.game();
        match action {
            Action::Home(tab) => {
                self.home_state.tab = tab;
                self.reload_home();
                self.catalog.refresh(&self.home);
            }
            Action::SettingsTab(tab) => {
                self.home_state.settings_tab = tab;
                self.catalog.scroll = 0.0;
            }
            Action::Drawer => self.home_state.drawer ^= true,
            Action::Link(name) => {
                let url = match name {
                    "repository" => "https://github.com/Vasysik/horizon-android".to_owned(),
                    name => links()[name].as_str().unwrap_or_default().to_owned(),
                };
                if url.is_empty() || !crate::platform::open_url(&url) {
                    self.catalog.status = "No browser available to open that link.".into();
                }
            }
            Action::SelectInstance(id) => {
                if id != self.catalog.config.profile {
                    self.select_instance(id)?;
                }
                self.home_state.drawer = false;
            }
            Action::NewInstance => {
                self.home_state.drawer = false;
                self.home_state.form =
                    Some((FormKind::NewInstance, String::new(), String::new(), 0));
            }
            Action::EditInstance => {
                let id = self.catalog.config.profile;
                let meta = self.home_state.instances.list.get(&id).cloned();
                let meta = meta.unwrap_or_default();
                self.home_state.menu = None;
                self.home_state.form =
                    Some((FormKind::EditInstance, meta.name, String::new(), meta.hue));
            }
            Action::CopyInstance => {
                let id = self.catalog.config.profile;
                let name = self
                    .home_state
                    .instances
                    .list
                    .get(&id)
                    .map_or("Horizon".to_owned(), |m| format!("{} copy", m.name));
                self.home_state.form = Some((FormKind::CopyInstance, name, String::new(), 0));
            }
            Action::DeleteInstance => {
                let id = self.catalog.config.profile;
                let name = self
                    .home_state
                    .instances
                    .list
                    .get(&id)
                    .map_or(String::new(), |m| m.name.clone());
                self.home_state.form = None;
                self.home_state.confirm = Some(Confirm {
                    title: format!("Delete {name}?"),
                    note: "Its worlds, packs and options go with it. Android has no trash here."
                        .into(),
                    label: "Delete".into(),
                    action: Action::ConfirmDeleteInstance,
                });
            }
            Action::ConfirmDeleteInstance => {
                let id = self.catalog.config.profile;
                self.home_state.confirm = None;
                self.select_instance(0)?;
                let home = self.home.clone();
                let result = self.home_state.instances.delete(&home, id);
                self.catalog.status = match result {
                    Ok(()) => "Instance deleted.".into(),
                    Err(error) => error,
                };
                self.reload_home();
            }
            Action::InstanceHue(i) => {
                if let Some(form) = &mut self.home_state.form {
                    form.3 = i;
                }
            }
            Action::SaveInstance => self.save_form()?,
            Action::SaveServer => {
                let Some((_, name, address, _)) = self.home_state.form.take() else {
                    return Ok(true);
                };
                self.catalog.editing = None;
                crate::platform::hide();
                let mut list = crate::server_list::List::load(&game);
                let name = if name.trim().is_empty() {
                    "Minecraft Server"
                } else {
                    name.trim()
                };
                self.catalog.status = match list
                    .add(name, address.trim())
                    .and_then(|()| list.save(&game))
                {
                    Ok(()) => format!("Added {name}."),
                    Err(error) => error,
                };
                self.reload_home();
            }
            Action::WorldMenu(i) => self.world_menu(i),
            Action::PlayWorld(i) => {
                self.home_state.menu = None;
                if let Some(world) = self.home_state.worlds.get(i) {
                    self.home_state.once_world = Some(world.folder.clone());
                    self.action(events)?;
                }
            }
            Action::BackupWorld(i) => {
                self.home_state.menu = None;
                if let Some(world) = self.home_state.worlds.get(i).cloned() {
                    let name = world.name.clone();
                    self.task(&format!("Backing up {name}…"), move || {
                        let file = crate::worlds::backup(&game, &world)?;
                        Ok(format!(
                            "Backed up {name}: {} in Backups.",
                            file.file_name().unwrap_or_default().to_string_lossy()
                        ))
                    });
                }
            }
            Action::CopyWorld(i) => {
                self.home_state.menu = None;
                if let Some(world) = self.home_state.worlds.get(i).cloned() {
                    self.task(&format!("Copying {}…", world.name), move || {
                        crate::worlds::copy(&world).map(|w| format!("Copied to {}.", w.name))
                    });
                }
            }
            Action::DeleteWorld(i) => {
                self.home_state.menu = None;
                if let Some(world) = self.home_state.worlds.get(i) {
                    self.home_state.pending = i;
                    self.home_state.confirm = Some(Confirm {
                        title: format!("Delete {}?", world.name),
                        note: "It's gone for good: Android has no trash here. Back it up first if you might want it."
                            .into(),
                        label: "Delete".into(),
                        action: Action::ConfirmDeleteWorld,
                    });
                }
            }
            Action::ConfirmDeleteWorld => {
                self.home_state.confirm = None;
                if let Some(world) = self.home_state.worlds.get(self.home_state.pending).cloned() {
                    self.task(&format!("Deleting {}…", world.name), move || {
                        crate::worlds::delete(&world).map(|()| format!("Deleted {}.", world.name))
                    });
                }
            }
            Action::WorldPacks(i) => self.pack_menu(i),
            Action::RemoveDataPack(i) => {
                let h = &mut self.home_state;
                h.menu = None;
                if let Some(pack) = h.packs.get(i) {
                    h.pending = i;
                    h.confirm = Some(Confirm {
                        title: format!("Remove {}?", pack.file),
                        note: "It's deleted from this world.".into(),
                        label: "Remove".into(),
                        action: Action::ConfirmRemoveDataPack,
                    });
                }
            }
            Action::ConfirmRemoveDataPack => {
                let h = &mut self.home_state;
                h.confirm = None;
                if let (Some(pack), Some(world)) = (h.packs.get(h.pending), h.worlds.get(h.pack_world))
                {
                    self.catalog.status = match crate::datapacks::remove(&world.path, pack) {
                        Ok(()) => format!("Removed {}.", pack.file),
                        Err(error) => error,
                    };
                }
            }
            Action::PickDataPackWorld(0) => {
                let rows = self
                    .home_state
                    .worlds
                    .iter()
                    .enumerate()
                    .map(|(i, w)| MenuRow {
                        icon: "earth",
                        label: w.name.clone(),
                        action: Action::PickDataPackWorld(i + 1),
                        danger: false,
                    })
                    .collect();
                self.home_state.menu = Some(Menu {
                    title: "Into which world?".into(),
                    note: String::new(),
                    rows,
                });
            }
            Action::PickDataPackWorld(n) => {
                self.home_state.menu = None;
                self.home_state.data_world = Some(n - 1);
            }
            Action::WorldsFolder => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "worlds");
                self.catalog.status = if crate::platform::open_documents(&id) {
                    "Copy complete world folders while the game is closed.".into()
                } else {
                    "Open system Files > Horizon Android > Worlds.".into()
                };
            }
            Action::BackupsFolder => {
                let id = crate::document_store::folder_id(self.catalog.config.profile, "backups");
                if !crate::platform::open_documents(&id) {
                    self.catalog.status = "Open system Files > Horizon Android > Backups.".into();
                }
            }
            Action::JoinServer(i) => {
                if let Some(server) = self.home_state.servers.get(i) {
                    self.home_state.once_server = Some(server.ip.clone());
                    self.action(events)?;
                }
            }
            Action::AddServer => {
                self.home_state.form =
                    Some((FormKind::AddServer, String::new(), String::new(), 0));
            }
            Action::RemoveServer(i) => {
                if let Some(server) = self.home_state.servers.get(i) {
                    self.home_state.pending = i;
                    self.home_state.confirm = Some(Confirm {
                        title: format!("Remove {}?", server.name),
                        note: "It goes from the game's list too.".into(),
                        label: "Remove".into(),
                        action: Action::ConfirmRemoveServer,
                    });
                }
            }
            Action::ConfirmRemoveServer => {
                self.home_state.confirm = None;
                let mut list = crate::server_list::List::load(&game);
                self.catalog.status = match list
                    .remove(self.home_state.pending)
                    .and_then(|s| list.save(&game).map(|()| s))
                {
                    Ok(server) => format!("Removed {}.", server.name),
                    Err(error) => error,
                };
                self.reload_home();
            }
            Action::JoinFriend => {
                let code: String = self
                    .home_state
                    .friend
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                if horizon_friends::invite::parse(&code).is_some() {
                    self.home_state.once_server =
                        Some(format!("{}{code}", horizon_friends::ADDRESS_PREFIX));
                    self.catalog.editing = None;
                    crate::platform::hide();
                    self.action(events)?;
                }
            }
            Action::OpenShot(i) => self.home_state.viewer = Some(i),
            Action::ShotStep(next) => {
                let count = self.home_state.shots.len();
                if let Some(i) = &mut self.home_state.viewer
                    && count > 0
                {
                    *i = if next {
                        (*i + 1) % count
                    } else {
                        (*i + count - 1) % count
                    };
                }
            }
            Action::SaveShot => {
                if let Some(shot) = self
                    .home_state
                    .viewer
                    .and_then(|i| self.home_state.shots.get(i))
                {
                    self.catalog.status = match crate::gallery::save(&shot.path) {
                        Ok(()) => "Saving to the phone's pictures…".into(),
                        Err(error) => error,
                    };
                }
            }
            Action::DeleteShot => {
                if let Some(i) = self.home_state.viewer {
                    self.home_state.pending = i;
                    self.home_state.confirm = Some(Confirm {
                        title: "Delete this screenshot?".into(),
                        note: "It's gone for good: Android has no trash here.".into(),
                        label: "Delete".into(),
                        action: Action::ConfirmDeleteShot,
                    });
                }
            }
            Action::ConfirmDeleteShot => {
                let h = &mut self.home_state;
                h.confirm = None;
                if let Some(shot) = h.shots.get(h.pending) {
                    self.catalog.status = match crate::screenshots::delete(shot) {
                        Ok(()) => "Screenshot deleted.".into(),
                        Err(error) => error,
                    };
                }
                self.home_state.viewer = None;
                self.reload_home();
            }
            Action::CloseShot => self.home_state.viewer = None,
            Action::CopyLog => {
                crate::platform::clipboard_set(&data::log_lines().join("\n"));
                self.catalog.status = "Log copied.".into();
            }
            Action::RefreshReleases => self.load_releases(),
            Action::OpenRelease(i) => {
                if let Some(release) = self.home_state.releases.get(i) {
                    let url = release.download(abi()).to_owned();
                    if !crate::platform::open_url(&url) {
                        self.catalog.status = "No browser available to open that link.".into();
                    }
                }
            }
            Action::OpenNote(i) => {
                if !self.home_state.open_notes.insert(i) {
                    self.home_state.open_notes.remove(&i);
                }
            }
            Action::Accent(i) => {
                self.catalog.config.accent = ACCENTS.get(i).map_or(0, |(_, c)| *c);
                self.catalog.config.save(&self.home)?;
            }
            Action::PlainGlass => {
                self.catalog.config.plain_plates ^= true;
                self.catalog.config.save(&self.home)?;
                self.home_state.art = glass::Art::default();
            }
            Action::StillScene => {
                self.catalog.config.still_scene ^= true;
                self.catalog.config.save(&self.home)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn save_form(&mut self) -> Result<()> {
        let Some((kind, name, _, hue)) = self.home_state.form.clone() else {
            return Ok(());
        };
        self.catalog.editing = None;
        crate::platform::hide();
        let home = self.home.clone();
        let id = self.catalog.config.profile;
        let list = &mut self.home_state.instances;
        let result = match kind {
            FormKind::NewInstance => list.create(&home, &name, hue, None),
            FormKind::CopyInstance => list.create(&home, &name, hue, Some(id)),
            FormKind::EditInstance => list.rename(&home, id, &name, hue).map(|()| id),
            FormKind::AddServer => return Ok(()),
        };
        match result {
            Ok(created) => {
                self.home_state.form = None;
                if created != id {
                    self.select_instance(created)?;
                } else {
                    self.reload_home();
                }
            }
            Err(error) => self.catalog.status = error,
        }
        Ok(())
    }
}

impl Home {
    fn form_view(&self) -> Option<Form> {
        let (kind, a, b, hue) = self.form.as_ref()?;
        let cancel = ("Cancel".to_owned(), Action::Dismiss, Kind::Secondary);
        Some(match kind {
            FormKind::AddServer => Form {
                title: "Add server".into(),
                note: String::new(),
                fields: vec![
                    (
                        "Name".into(),
                        "server_name",
                        a.clone(),
                        "Minecraft Server".into(),
                    ),
                    (
                        "Address".into(),
                        "server_address",
                        b.clone(),
                        "play.example.net".into(),
                    ),
                ],
                hues: None,
                buttons: vec![
                    cancel,
                    ("Add".into(), Action::SaveServer, Kind::Primary),
                ],
            },
            kind => {
                let editing = *kind == FormKind::EditInstance;
                let mut buttons = vec![cancel];
                if editing {
                    buttons.push(("Copy".into(), Action::CopyInstance, Kind::Secondary));
                    buttons.push(("Delete".into(), Action::DeleteInstance, Kind::Danger));
                }
                buttons.push((
                    if editing { "Save" } else { "Create" }.into(),
                    Action::SaveInstance,
                    Kind::Primary,
                ));
                Form {
                    title: match kind {
                        FormKind::NewInstance => "New instance",
                        FormKind::CopyInstance => "Copy instance",
                        _ => "Edit instance",
                    }
                    .into(),
                    note: if editing {
                        String::new()
                    } else {
                        "Its own worlds, options and packs.".into()
                    },
                    fields: vec![(
                        "Name".into(),
                        "instance_name",
                        a.clone(),
                        "A name".into(),
                    )],
                    hues: Some(*hue),
                    buttons,
                }
            }
        })
    }
}
