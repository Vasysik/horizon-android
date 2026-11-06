//! The launcher's screens in the desktop launcher's glass look, laid out for a phone: the frame
//! (sidebar and top bar on a wide screen such as a Fold's inner display, a bottom bar on a narrow
//! one) and Home, Versions, Mods, Skins' frame, Patch notes and Settings. Everything here reads a
//! plain [`View`] and only says which [`Action`] a tap means, so `launcher_pages.rs` holds the
//! Android side and the preview example can draw the same screens on a computer.
use crate::launcher_glass::{
    Art, Kind, Pixels, Plate, Targets, clip, dot, fitted, paragraph, picture, tone, white, wrapped,
};
use crate::launcher_tokens::{self as tokens, fill, ink, radius};
use horizon_client::content_files::Kind as Content;
use horizon_client::ui_theme::{Face, Ui, color::hex};

/// The width from which the sidebar is shown beside the pages.
pub const WIDE: f32 = 720.;
const SIDEBAR: f32 = 232.;
const EDGE: f32 = 12.;
const TOP_BAR: f32 = 52.;
const BOTTOM_BAR: f32 = 62.;

/// The launcher's pages. Mods, Packs, Shaders and DataPacks are the browser's tabs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Page {
    Play,
    Versions,
    Mods,
    Packs,
    Shaders,
    DataPacks,
    Settings,
    Skins,
    Notes,
}
impl Page {
    /// The pages in the top bar, with their icons.
    pub const NAV: [(Self, &'static str, &'static str); 6] = [
        (Self::Play, "Home", "game-controller-01"),
        (Self::Versions, "Versions", "package-01"),
        (Self::Mods, "Mods", "puzzle"),
        (Self::Skins, "Skins", "user"),
        (Self::Notes, "Notes", "information-circle"),
        (Self::Settings, "Settings", "sliders-horizontal"),
    ];
    pub fn is_catalog(self) -> bool {
        matches!(
            self,
            Self::Packs | Self::Mods | Self::Shaders | Self::DataPacks
        )
    }
    fn in_nav(self, other: Self) -> bool {
        self == other || (self.is_catalog() && other == Self::Mods)
    }
}

/// Home's tabs, under the instance's name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum HomeTab {
    #[default]
    Overview,
    Mods,
    Worlds,
    Multiplayer,
    Packs,
    Screenshots,
    Console,
}

/// Settings' sections.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsTab {
    #[default]
    Launcher,
    Game,
    Screen,
    Reports,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    Page(Page),
    Home(HomeTab),
    SettingsTab(SettingsTab),
    Drawer,
    Play,
    Search,
    ShaderSource(crate::shader_catalog::Origin),
    ModSource(crate::shader_catalog::Origin),
    Field(&'static str),
    FinishNameEdit,
    Install(usize),
    Import(Content),
    OpenFolder(Content),
    ToggleLocal(usize),
    DeleteLocal(usize),
    ConfirmDeleteLocal,
    ConfirmExit,
    Next,
    Previous,
    Logs,
    RecordPerformance,
    ExportPerformance,
    ExportDiagnostics,
    Account,
    SignIn,
    ElySignIn,
    MicrosoftAccount,
    OfflineAccount,
    SubmitEly,
    SavedAccounts,
    SavedPage(bool),
    SelectAccount(usize),
    SelectOffline,
    WardrobeKind(bool),
    WardrobeSelect(usize),
    WardrobeImport,
    WardrobeCloud,
    WardrobeUpload,
    WardrobeActivateCape(usize),
    WardrobeUseCloud,
    WardrobeModel,
    WardrobeRotate(i8),
    WardrobeZoom(bool),
    WardrobeHideCape,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    Dismiss,
    Immersive,
    Cutout,
    Aspect,
    ChooseAspect(crate::display::Aspect),
    DisplayHz,
    ChooseDisplayHz(u16),
    CacheBudget,
    ChooseCacheBudget(u32),
    ToggleBundled(&'static str),
    GameFiles,
    WorkerLimit,
    ChooseWorkerLimit(u32),
    PerformanceHints,
    GpuProfile,
    // Instances.
    SelectInstance(u32),
    NewInstance,
    EditInstance,
    CopyInstance,
    DeleteInstance,
    ConfirmDeleteInstance,
    InstanceHue(usize),
    SaveInstance,
    // Worlds.
    WorldMenu(usize),
    PlayWorld(usize),
    BackupWorld(usize),
    CopyWorld(usize),
    DeleteWorld(usize),
    ConfirmDeleteWorld,
    WorldPacks(usize),
    RemoveDataPack(usize),
    ConfirmRemoveDataPack,
    WorldsFolder,
    BackupsFolder,
    PickDataPackWorld(usize),
    // Multiplayer.
    JoinServer(usize),
    AddServer,
    SaveServer,
    RemoveServer(usize),
    ConfirmRemoveServer,
    JoinFriend,
    // Screenshots.
    OpenShot(usize),
    ShotStep(bool),
    SaveShot,
    DeleteShot,
    ConfirmDeleteShot,
    CloseShot,
    // Console.
    CopyLog,
    // Versions and notes.
    RefreshReleases,
    OpenRelease(usize),
    OpenNote(usize),
    // Settings.
    Accent(usize),
    PlainGlass,
    StillScene,
    Link(&'static str),
}

/// One instance in the sidebar.
#[derive(Clone, Default)]
pub struct InstanceRow {
    pub id: u32,
    pub name: String,
    pub note: String,
    pub hue: u32,
    pub selected: bool,
}

/// What Play can do now.
#[derive(Clone, Default)]
pub struct Game {
    /// The game's files are here and verified.
    pub installed: bool,
    pub installing: bool,
    /// The install's steps: (label, done, current).
    pub steps: Vec<(String, bool, bool)>,
    pub overall: f32,
    pub detail: String,
    pub stage: String,
    pub failed: bool,
    /// Waiting for an account's session; Play can't be pressed again.
    pub preparing: bool,
}

/// A row of Home's lists.
#[derive(Clone, Default)]
pub struct WorldRow {
    pub name: String,
    pub note: String,
    /// Its `icon.png`, already decoded: key and pixels.
    pub icon: Option<(u64, Pixels)>,
}
#[derive(Clone, Default)]
pub struct ServerRow {
    pub name: String,
    pub address: String,
    pub icon: Option<(u64, Pixels)>,
}
#[derive(Clone)]
pub struct LocalRow {
    pub index: usize,
    pub kind: Content,
    pub title: String,
    pub enabled: bool,
}
#[derive(Clone)]
pub struct BundledRow {
    pub id: &'static str,
    pub title: String,
    pub note: String,
    pub enabled: bool,
}
#[derive(Clone, Default)]
pub struct ShotRow {
    pub name: String,
    pub thumb: Option<(u64, Pixels)>,
}
/// A result of the browser.
#[derive(Clone, Default)]
pub struct ProjectRow {
    pub index: usize,
    pub title: String,
    pub note: String,
    /// What stands where Install would: "Installed", "42%", "Java mod".
    pub state: Option<String>,
    pub install: Option<String>,
}
/// A release in Versions and Patch notes.
#[derive(Clone, Default)]
pub struct ReleaseRow {
    pub name: String,
    pub note: String,
    pub body: String,
    pub open: bool,
    pub current: bool,
}

/// Settings' values, as they read.
#[derive(Clone, Default)]
pub struct SettingsView {
    pub tab: SettingsTab,
    pub accent: usize,
    pub plain: bool,
    pub still: bool,
    pub renderer: String,
    pub resolution: String,
    pub workers: String,
    pub cache: String,
    pub orientation: String,
    pub input: String,
    pub immersive: bool,
    pub cutout: bool,
    pub aspect: String,
    pub display: String,
    pub hints: bool,
    pub gpu_profile: bool,
    pub recording: bool,
    pub game_args: String,
    pub proxy: String,
    pub server: String,
    pub device: String,
}

/// Everything a frame shows.
#[derive(Clone)]
pub struct View {
    pub version: String,
    pub minecraft: String,
    pub page: Page,
    pub home_tab: HomeTab,
    pub account_name: String,
    pub account_how: String,
    pub instances: Vec<InstanceRow>,
    pub played: String,
    pub game: Game,
    pub status: String,
    pub editing: Option<&'static str>,
    pub friend: String,
    pub friend_valid: bool,
    pub bits: bool,
    pub worlds: Vec<WorldRow>,
    pub servers: Vec<ServerRow>,
    pub local: Vec<LocalRow>,
    pub bundled: Vec<BundledRow>,
    pub shots: Vec<ShotRow>,
    pub log: Vec<String>,
    pub query: String,
    pub source: Option<crate::shader_catalog::Origin>,
    pub search_hint: String,
    pub results: Vec<ProjectRow>,
    pub offset: usize,
    pub data_world: Option<String>,
    pub releases: Vec<ReleaseRow>,
    pub releases_note: String,
    pub settings: SettingsView,
}

/// A frame's tappable places and what scrolls.
#[derive(Default)]
pub struct Out {
    pub targets: Targets<Action>,
    /// The scrolled page's places before they were cut to its window, for a controller's focus.
    pub list: Targets<Action>,
    /// The scrolled window's top and bottom, and how far it scrolls.
    pub scroll: Option<[f32; 3]>,
}

/// The page's room on the screen.
#[derive(Clone, Copy)]
pub struct Area {
    pub rect: [f32; 4],
    /// Beside the sidebar, or the whole width of a narrow screen.
    pub wide: bool,
}

pub fn area(ui: &Ui) -> Area {
    let wide = ui.w >= WIDE;
    Area {
        wide,
        rect: if wide {
            let x = EDGE + SIDEBAR + EDGE;
            [
                x,
                EDGE + TOP_BAR + 8.,
                ui.w - x - EDGE,
                ui.h - EDGE - TOP_BAR - 8.,
            ]
        } else {
            [
                EDGE,
                EDGE + 48. + 8.,
                ui.w - 2. * EDGE,
                ui.h - EDGE - 48. - 8. - BOTTOM_BAR - 8. - 8.,
            ]
        },
    }
}

/// A dialog's or sheet's title.
pub fn head(ui: &mut Ui, x: f32, y: f32, w: f32, title: &str, note: &str) -> f32 {
    let lines = wrapped(ui, Face::Display, 26., title, w);
    for (i, l) in lines.iter().enumerate() {
        ui.text(
            Face::Display,
            26.,
            x,
            y + i as f32 * 32.,
            white(ink::FULL),
            l,
        );
    }
    let mut bottom = y + lines.len() as f32 * 32. + 2.;
    if !note.is_empty() {
        bottom += paragraph(ui, Face::Regular, 14., x, bottom, w, white(ink::BODY), note) + 4.;
    }
    bottom
}

/// A page's head: its name in Bricolage, the note under it.
fn page_head(ui: &mut Ui, area: [f32; 4], y: f32, title: &str, note: &str) -> f32 {
    let size = if area[2] < 520. { 34. } else { 46. };
    ui.text(
        Face::Display,
        size,
        area[0] + 4.,
        y,
        white(ink::FULL),
        title,
    );
    let mut bottom = y + size * 1.45;
    if !note.is_empty() {
        bottom += paragraph(
            ui,
            Face::Regular,
            15.,
            area[0] + 4.,
            bottom + 2.,
            (area[2] - 8.).min(615.),
            white(ink::BODY),
            note,
        ) + 2.;
    }
    bottom + 18.
}

/// Draws `plate` under what was drawn since `start`.
fn under(ui: &mut Ui, start: usize, plate: impl FnOnce(&mut Ui)) {
    let before = ui.batch.vertices.len();
    plate(ui);
    let added = ui.batch.vertices.len() - before;
    ui.batch.vertices[start..].rotate_right(added);
}

fn card_heading(ui: &mut Ui, x: f32, y: f32, text: &str) -> f32 {
    ui.text(Face::Medium, 13., x, y, white(ink::BODY), text);
    y + 26.
}

fn empty_note(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    y + paragraph(ui, Face::Regular, 13., x, y, w, white(ink::QUIET), text) + 8.
}

fn rule(ui: &mut Ui, x: f32, y: f32, w: f32) {
    ui.rounded(x, y, w, 1., 0., white(0.08));
}

/// A list row: a tile, two lines, and `trailing` px of room on the right for its buttons.
#[allow(clippy::too_many_arguments)]
fn list_row(
    art: &Art,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    trailing: f32,
    icon: Option<&(u64, Pixels)>,
    fallback: &str,
    name: &str,
    note: &str,
) {
    const SIDE: f32 = 40.;
    let top = y + 10.;
    if let Some((key, pixels)) = icon {
        picture(ui, *key, pixels, [x, top, SIDE, SIDE]);
    } else {
        ui.rounded(x, top, SIDE, SIDE, radius::ROW, white(fill::SELECTED));
        art.icon(ui, fallback, x + 10., top + 10., 20., white(ink::SOFT));
    }
    let room = w - SIDE - 12. - trailing;
    fitted(
        ui,
        Face::SemiBold,
        14.,
        x + SIDE + 12.,
        top,
        room,
        white(ink::FULL),
        name,
    );
    fitted(
        ui,
        Face::Regular,
        12.5,
        x + SIDE + 12.,
        top + 22.,
        room,
        white(ink::MUTED),
        note,
    );
}

const ROW: f32 = 60.;

impl View {
    /// The frame around the page, then the page.
    pub fn draw(&self, art: &Art, ui: &mut Ui, scroll: f32, out: &mut Out) {
        let region = area(ui);
        if region.wide {
            self.sidebar(art, ui, [EDGE, EDGE, SIDEBAR, ui.h - 2. * EDGE], out);
            self.top_bar(art, ui, region.rect[0], out);
        } else {
            self.narrow_top(art, ui, out);
            self.bottom_bar(art, ui, out);
        }
        let start = ui.batch.vertices.len();
        let first = out.targets.len();
        let rect = region.rect;
        let height = match self.page {
            Page::Play => self.home(art, ui, rect, rect[1] + 8. - scroll, &mut out.targets),
            Page::Versions => self.versions(art, ui, rect, rect[1] + 8. - scroll, &mut out.targets),
            Page::Notes => self.notes(art, ui, rect, rect[1] + 8. - scroll, &mut out.targets),
            Page::Settings => self.settings(art, ui, rect, rect[1] + 8. - scroll, &mut out.targets),
            Page::Skins => 0.,
            _ => self.browser(art, ui, rect, rect[1] + 8. - scroll, &mut out.targets),
        };
        if self.page != Page::Skins {
            self.finish_page(ui, start, first, rect, height + scroll, out);
        }
        self.status_line(ui, rect);
    }

    /// Cuts the page to its window and records how far it scrolls.
    pub fn finish_page(
        &self,
        ui: &mut Ui,
        start: usize,
        first: usize,
        rect: [f32; 4],
        bottom: f32,
        out: &mut Out,
    ) {
        clip(ui, start, rect);
        out.list = out.targets[first..].to_vec();
        let (top, end) = (rect[1], rect[1] + rect[3]);
        let mut kept = out.targets.split_off(first);
        kept.retain_mut(|(t, _)| {
            let y0 = t[1].max(top);
            let y1 = (t[1] + t[3]).min(end);
            if y1 - y0 < 8. {
                return false;
            }
            t[1] = y0;
            t[3] = y1 - y0;
            true
        });
        out.targets.append(&mut kept);
        out.scroll = Some([top, end, (bottom + 24. - end).max(0.)]);
    }

    fn status_line(&self, ui: &mut Ui, rect: [f32; 4]) {
        if self.status.is_empty() {
            return;
        }
        let w = (rect[2] - 16.).min(640.);
        let lines = wrapped(ui, Face::Regular, 13., &self.status, w - 28.);
        let lines = &lines[..lines.len().min(3)];
        let h = 16. + lines.len() as f32 * 19.;
        let x = rect[0] + (rect[2] - w) / 2.;
        let y = rect[1] + rect[3] - h - 6.;
        ui.rounded(x, y, w, h, radius::ROW, hex(tokens::SCRIM, 225));
        for (i, l) in lines.iter().enumerate() {
            ui.text(
                Face::Regular,
                13.,
                x + 14.,
                y + 7. + i as f32 * 19.,
                white(ink::STRONG),
                l,
            );
        }
    }

    fn sidebar(&self, art: &Art, ui: &mut Ui, rect: [f32; 4], out: &mut Out) {
        let [x, y, w, h] = rect;
        art.plate(ui, rect, radius::PANEL, Plate::Glass);
        let pad = 12.;
        art.mark(ui, x + pad + 2., y + 18., 22.);
        ui.text(
            Face::Display,
            22.,
            x + pad + 32.,
            y + 13.,
            white(ink::FULL),
            "Horizon",
        );
        let mut row = y + 56.;
        ui.text(
            Face::Medium,
            11.,
            x + pad + 4.,
            row + 6.,
            white(ink::QUIET),
            "INSTANCES",
        );
        art.icon_button(
            ui,
            &mut out.targets,
            [x + w - pad - 36., row - 4., 36., 36.],
            "plus-sign",
            Action::NewInstance,
            false,
        );
        row += 36.;
        let foot = y + h - 138.;
        for i in &self.instances {
            if row + 58. > foot {
                break;
            }
            let r = [x + 8., row, w - 16., 56.];
            if i.selected {
                ui.rounded(r[0], r[1], r[2], r[3], radius::ROW, white(fill::SELECTED));
            }
            art.tile(ui, r[0] + 8., r[1] + 8., 40., i.hue, &i.name);
            fitted(
                ui,
                Face::SemiBold,
                14.,
                r[0] + 58.,
                r[1] + 7.,
                r[2] - 66.,
                white(ink::FULL),
                &i.name,
            );
            dot(ui, r[0] + 58., r[1] + 34., tokens::LIVE);
            fitted(
                ui,
                Face::Regular,
                12.5,
                r[0] + 70.,
                r[1] + 29.,
                r[2] - 78.,
                white(ink::MUTED),
                &i.note,
            );
            out.targets.push((r, Action::SelectInstance(i.id)));
            row += 58.;
        }
        let mut link_y = y + h - 134.;
        for (label, icon, link) in [
            ("Discord", "discord", "discord"),
            ("Website", "globe-02", "website"),
            ("GitHub", "github", "repository"),
        ] {
            let r = [x + 8., link_y, w - 16., 34.];
            art.icon(ui, icon, r[0] + 8., r[1] + 8., 18., white(ink::MUTED));
            ui.text(
                Face::Regular,
                14.,
                r[0] + 36.,
                r[1] + 7.,
                white(ink::BODY),
                label,
            );
            out.targets.push((r, Action::Link(link)));
            link_y += 34.;
        }
        ui.text(
            Face::Regular,
            12.,
            x + pad + 4.,
            y + h - 28.,
            white(ink::QUIET),
            &format!("Horizon Android {}", self.version),
        );
    }

    fn account_chip(&self, art: &Art, ui: &mut Ui, r: [f32; 4], out: &mut Out) {
        let [x, y, w, h] = r;
        art.plate(ui, r, radius::ROW, Plate::Glass);
        ui.rounded(x + 8., y + 8., h - 16., h - 16., 8., white(fill::SELECTED));
        art.icon(
            ui,
            "user",
            x + 8. + (h - 16. - 18.) / 2.,
            y + 8. + (h - 16. - 18.) / 2.,
            18.,
            white(ink::SOFT),
        );
        let left = x + h;
        let room = w - h - 8.;
        fitted(
            ui,
            Face::SemiBold,
            13.,
            left,
            y + (h - 36.) / 2.,
            room,
            white(ink::FULL),
            &self.account_name,
        );
        fitted(
            ui,
            Face::Regular,
            12.,
            left,
            y + (h - 36.) / 2. + 18.,
            room,
            white(ink::MUTED),
            &self.account_how,
        );
        out.targets.push((r, Action::Account));
    }

    fn top_bar(&self, art: &Art, ui: &mut Ui, left: f32, out: &mut Out) {
        let chip = 190.;
        let right = ui.w - EDGE;
        let strip = [left, EDGE, right - left - chip - 8., TOP_BAR];
        art.plate(ui, strip, radius::LARGE, Plate::Glass);
        let mut x = left + 8.;
        for (page, label, icon) in Page::NAV {
            let compact = strip[2] < 620.;
            x += art.tab(
                ui,
                &mut out.targets,
                x,
                EDGE + 8.,
                36.,
                (!compact).then_some(icon),
                label,
                None,
                self.page.in_nav(page),
                Action::Page(page),
            ) + 4.;
        }
        self.account_chip(art, ui, [right - chip, EDGE, chip, TOP_BAR], out);
    }

    fn narrow_top(&self, art: &Art, ui: &mut Ui, out: &mut Out) {
        let w = ui.w;
        let selected = self.instances.iter().find(|i| i.selected);
        let left = [EDGE, EDGE, w - 2. * EDGE - 158., 48.];
        art.plate(ui, left, radius::LARGE, Plate::Glass);
        art.mark(ui, left[0] + 12., left[1] + 13., 22.);
        fitted(
            ui,
            Face::SemiBold,
            14.,
            left[0] + 44.,
            left[1] + 14.,
            left[2] - 72.,
            white(ink::FULL),
            selected.map_or("Horizon", |i| i.name.as_str()),
        );
        art.icon(
            ui,
            "arrow-down-01",
            left[0] + left[2] - 28.,
            left[1] + 15.,
            18.,
            white(ink::MUTED),
        );
        out.targets.push((left, Action::Drawer));
        self.account_chip(art, ui, [w - EDGE - 150., EDGE, 150., 48.], out);
    }

    fn bottom_bar(&self, art: &Art, ui: &mut Ui, out: &mut Out) {
        let r = [EDGE, ui.h - EDGE - BOTTOM_BAR, ui.w - 2. * EDGE, BOTTOM_BAR];
        art.plate(ui, r, radius::PANEL, Plate::Glass);
        let each = (r[2] - 8.) / Page::NAV.len() as f32;
        for (i, (page, label, icon)) in Page::NAV.into_iter().enumerate() {
            let x = r[0] + 4. + i as f32 * each;
            let open = self.page.in_nav(page);
            if open {
                ui.rounded(
                    x + 2.,
                    r[1] + 5.,
                    each - 4.,
                    BOTTOM_BAR - 10.,
                    radius::ROW,
                    white(fill::ACTIVE),
                );
            }
            let c = white(if open { ink::FULL } else { ink::MUTED });
            art.icon(ui, icon, x + (each - 22.) / 2., r[1] + 10., 22., c);
            let lw = ui.measure(Face::Regular, 11., label);
            ui.text(
                Face::Regular,
                11.,
                x + (each - lw) / 2.,
                r[1] + 35.,
                c,
                label,
            );
            out.targets
                .push(([x, r[1], each, BOTTOM_BAR], Action::Page(page)));
        }
    }

    /// The instances over the page on a narrow screen.
    pub fn drawer(&self, art: &Art, ui: &mut Ui, out: &mut Out) {
        out.targets.clear();
        out.list.clear();
        out.scroll = None;
        scrim(ui);
        out.targets.push(([0., 0., ui.w, ui.h], Action::Drawer));
        let w = (ui.w - 56.).min(300.);
        self.sidebar(art, ui, [EDGE, EDGE, w, ui.h - 2. * EDGE], out);
    }

    fn home(&self, art: &Art, ui: &mut Ui, area: [f32; 4], y: f32, t: &mut Targets<Action>) -> f32 {
        let [x, _, w, _] = area;
        let selected = self.instances.iter().find(|i| i.selected);
        let name = selected.map_or("Horizon", |i| i.name.as_str());
        let size = if w < 520. { 36. } else { 46. };
        let mut y = y + 8.;
        fitted(
            ui,
            Face::Display,
            size,
            x + 4.,
            y,
            w - 56.,
            white(ink::FULL),
            name,
        );
        art.icon_button(
            ui,
            t,
            [x + w - 44., y + size * 0.3, 40., 40.],
            "pencil-edit-02",
            Action::EditInstance,
            false,
        );
        y += size * 1.4;
        ui.text(
            Face::Regular,
            14.,
            x + 4.,
            y,
            white(ink::MUTED),
            &format!("Minecraft {}  ·  {}", self.minecraft, self.played),
        );
        y += 36.;
        let tall = 56.;
        let narrow = w < 620.;
        let play_w = if narrow { w - 66. } else { 220. };
        let g = &self.game;
        let (label, kind) = if g.installing {
            (g.stage.as_str(), Kind::Off)
        } else if g.preparing {
            ("Preparing…", Kind::Off)
        } else if g.installed {
            ("Play", Kind::Primary)
        } else if g.failed {
            ("Retry download", Kind::Primary)
        } else {
            ("Download and play", Kind::Primary)
        };
        ui.rounded(x, y, play_w, tall, radius::LARGE, 0);
        art.icon_label(
            ui,
            t,
            [x, y, play_w, tall],
            Some(if g.installed { "play" } else { "download-04" }),
            label,
            Action::Play,
            kind,
        );
        let folder = [x + play_w + 10., y, tall, tall];
        ui.rounded(
            folder[0],
            folder[1],
            tall,
            tall,
            radius::LARGE,
            white(fill::BUTTON),
        );
        art.icon_button(ui, t, folder, "folder-01", Action::GameFiles, false);
        let (field_x, field_y) = if narrow {
            (x, y + tall + 10.)
        } else {
            (folder[0] + tall + 10., y)
        };
        let field_w = x + w - field_x;
        self.friend_field(art, ui, t, [field_x, field_y, field_w, tall]);
        y = field_y + tall + 20.;
        let panel_top = y;
        let start = ui.batch.vertices.len();
        let mut tab_x = x + 8.;
        let mut tab_y = y + 8.;
        let tabs = [
            (HomeTab::Overview, "dashboard-square-01", "Overview", None),
            (HomeTab::Mods, "puzzle", "Mods", Some(self.mods_count())),
            (HomeTab::Worlds, "earth", "Worlds", Some(self.worlds.len())),
            (
                HomeTab::Multiplayer,
                "user-multiple-02",
                "Multiplayer",
                Some(self.servers.len()),
            ),
            (HomeTab::Packs, "", "Packs", Some(self.packs_count())),
            (
                HomeTab::Screenshots,
                "",
                "Screenshots",
                Some(self.shots.len()),
            ),
            (HomeTab::Console, "computer-terminal-01", "Console", None),
        ];
        let measure = |ui: &Ui, icons: bool| -> f32 {
            tabs.iter()
                .map(|(_, icon, label, count)| {
                    28. + if icons && !icon.is_empty() { 22. } else { 0. }
                        + ui.measure(Face::Medium, 14., label)
                        + count
                            .filter(|n| *n > 0)
                            .map_or(0., |n| ui.measure(Face::Regular, 12., &n.to_string()) + 8.)
                })
                .sum()
        };
        let icons = w < 620. || measure(ui, true) <= w - 16.;
        for (tab, icon, label, count) in tabs {
            let icon = if icons { icon } else { "" };
            let width = 24.
                + if icon.is_empty() { 0. } else { 22. }
                + ui.measure(Face::Medium, 14., label)
                + count
                    .filter(|n| *n > 0)
                    .map_or(0., |n| ui.measure(Face::Regular, 12., &n.to_string()) + 8.);
            if tab_x + width > x + w - 8. {
                tab_x = x + 8.;
                tab_y += 42.;
            }
            tab_x += art.tab(
                ui,
                t,
                tab_x,
                tab_y,
                38.,
                (!icon.is_empty()).then_some(icon),
                label,
                count,
                self.home_tab == tab,
                Action::Home(tab),
            ) + 4.;
        }
        let body = [x + 12., tab_y + 50., w - 24., 0.];
        let end = match self.home_tab {
            HomeTab::Overview => self.overview(art, ui, body, t),
            HomeTab::Mods => self.home_mods(art, ui, body, t),
            HomeTab::Worlds => self.home_worlds(art, ui, body, t),
            HomeTab::Multiplayer => self.home_servers(art, ui, body, t),
            HomeTab::Packs => self.home_packs(art, ui, body, t),
            HomeTab::Screenshots => self.home_shots(art, ui, body, t),
            HomeTab::Console => self.home_console(art, ui, body, t),
        };
        let height = (end + 12. - panel_top).max(300.);
        under(ui, start, |ui| {
            art.plate(ui, [x, panel_top, w, height], radius::PANEL, Plate::Glass);
        });
        panel_top + height
    }

    fn mods_count(&self) -> usize {
        self.local
            .iter()
            .filter(|l| l.kind == Content::Mods)
            .count()
            + self.bundled.iter().filter(|b| b.enabled).count()
    }
    fn packs_count(&self) -> usize {
        self.local
            .iter()
            .filter(|l| l.kind != Content::Mods)
            .count()
    }

    fn friend_field(&self, art: &Art, ui: &mut Ui, t: &mut Targets<Action>, r: [f32; 4]) {
        let focused = self.editing == Some("friend");
        art.field(
            ui,
            t,
            [
                r[0],
                r[1],
                r[2] - if self.friend_valid { 76. } else { 0. },
                r[3],
            ],
            Some("user-add-01"),
            &self.friend,
            "Paste a friend's invite code",
            focused,
            Action::Field("friend"),
        );
        if self.friend_valid {
            art.button(
                ui,
                t,
                [r[0] + r[2] - 68., r[1] + 8., 68., r[3] - 16.],
                "Join",
                Action::JoinFriend,
                Kind::Primary,
            );
        }
    }

    fn overview(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = y;
        let g = &self.game;
        if g.installing {
            y = card_heading(ui, x, y, "Getting your game ready");
            for (label, done, current) in &g.steps {
                let c = if *current {
                    white(ink::FULL)
                } else {
                    white(ink::MUTED)
                };
                if *done {
                    art.icon(ui, "tick-02", x, y + 2., 18., hex(tokens::LIVE, 255));
                } else if *current {
                    dot(ui, x + 5., y + 8., tokens::WARN);
                }
                ui.text(Face::Regular, 14., x + 28., y, c, label);
                y += 26.;
            }
            y += 8.;
            art.bar(ui, [x, y, w, 5.], g.overall);
            y += 14.;
            y += paragraph(
                ui,
                Face::Regular,
                13.,
                x,
                y,
                w,
                white(ink::QUIET),
                &g.detail,
            );
            return y;
        }
        let wide = w >= 560.;
        let left_w = if wide { w - 300. } else { w };
        let shots_bottom = {
            let mut cy = card_heading(ui, x, y, "Last screenshots");
            if self.shots.is_empty() {
                cy = empty_note(
                    ui,
                    x,
                    cy,
                    left_w,
                    "F2 in the game saves one. They show here and in the Screenshots tab.",
                );
            } else {
                let columns = ((left_w / 200.).ceil() as usize).clamp(3, 6);
                let cols = columns as f32;
                let gap = 8.;
                let tw = (left_w - gap * (cols - 1.)) / cols;
                let th = tw * 9. / 16.;
                for (i, s) in self.shots.iter().take(columns * 2).enumerate() {
                    let (c, r) = ((i % columns) as f32, (i / columns) as f32);
                    let rect = [x + c * (tw + gap), cy + r * (th + gap), tw, th];
                    shot_tile(ui, s, rect);
                    t.push((rect, Action::OpenShot(i)));
                }
                cy += self.shots.len().min(columns * 2).div_ceil(columns) as f32 * (th + gap);
            }
            cy
        };
        let notes_x = if wide { x + left_w + 20. } else { x };
        let notes_w = if wide { 280. } else { w };
        let notes_y = if wide { y } else { shots_bottom + 8. };
        let mut cy = card_heading(ui, notes_x, notes_y, "Patch notes");
        if let Some(r) = self.releases.first() {
            fitted(
                ui,
                Face::SemiBold,
                14.,
                notes_x,
                cy,
                notes_w,
                white(ink::FULL),
                &r.name,
            );
            cy += 22.;
            let lines = wrapped(ui, Face::Regular, 13., &r.body, notes_w);
            for l in lines.iter().take(6) {
                fitted(
                    ui,
                    Face::Regular,
                    13.,
                    notes_x,
                    cy,
                    notes_w,
                    white(ink::BODY),
                    l,
                );
                cy += 19.;
            }
            art.button(
                ui,
                t,
                [notes_x, cy + 6., 120., 38.],
                "All notes",
                Action::Page(Page::Notes),
                Kind::Secondary,
            );
            cy += 50.;
        } else {
            cy = empty_note(ui, notes_x, cy, notes_w, &self.releases_note);
        }
        let mut y = shots_bottom.max(cy) + 8.;
        rule(ui, x, y, w);
        y += 8.;
        let (state, action): (&str, Option<(&str, Action)>) = if g.installed {
            ("Ready. Files are verified on this device.", None)
        } else if g.failed {
            (
                "The last download stopped. Finished files are kept.",
                Some(("Retry", Action::Play)),
            )
        } else {
            (
                "Horizon downloads the game's files from Mojang onto this device.",
                Some(("Download", Action::Play)),
            )
        };
        ui.text(
            Face::Medium,
            14.,
            x,
            y + 10.,
            white(ink::STRONG),
            "Resources",
        );
        let note_x = if wide { x + 140. } else { x };
        let note_y = if wide { y + 10. } else { y + 34. };
        let room = w - (note_x - x) - if action.is_some() { 110. } else { 0. };
        let h = paragraph(
            ui,
            Face::Regular,
            14.,
            note_x,
            note_y,
            room,
            white(ink::BODY),
            state,
        );
        if let Some((label, a)) = action {
            art.button(
                ui,
                t,
                [x + w - 100., y + 4., 100., 40.],
                label,
                a,
                Kind::Secondary,
            );
        }
        (note_y + h).max(y + 48.) + 8.
    }

    fn local_rows(
        &self,
        art: &Art,
        ui: &mut Ui,
        x: f32,
        mut y: f32,
        w: f32,
        kind: Content,
        t: &mut Targets<Action>,
    ) -> f32 {
        for l in self.local.iter().filter(|l| l.kind == kind) {
            let icon = match kind {
                Content::Mods => "puzzle",
                Content::Packs => "image-01",
                Content::Shaders => "color-picker",
            };
            list_row(
                art,
                ui,
                x,
                y,
                w,
                112.,
                None,
                icon,
                &l.title,
                if l.enabled { "On" } else { "Off" },
            );
            if kind == Content::Mods && !self.bits {
                ui.text(
                    Face::Regular,
                    12.,
                    x + w - 150.,
                    y + 20.,
                    white(ink::QUIET),
                    "Needs a 64-bit APK",
                );
            } else {
                art.toggle(
                    ui,
                    t,
                    [x + w - 106., y + 8., 56., 44.],
                    l.enabled,
                    Action::ToggleLocal(l.index),
                );
            }
            art.icon_button(
                ui,
                t,
                [x + w - 44., y + 8., 44., 44.],
                "delete-02",
                Action::DeleteLocal(l.index),
                false,
            );
            y += ROW;
        }
        y
    }

    fn home_mods(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = list_head(
            art,
            ui,
            t,
            x,
            y,
            w,
            "Sandboxed .hmod mods. Changes apply on the next Play.",
            &[
                ("Find mods", "search-01", Action::Page(Page::Mods)),
                ("Add file", "plus-sign", Action::Import(Content::Mods)),
            ],
        );
        for m in &self.bundled {
            list_row(art, ui, x, y, w, 60., None, "puzzle", &m.title, &m.note);
            art.toggle(
                ui,
                t,
                [x + w - 56., y + 8., 56., 44.],
                m.enabled,
                Action::ToggleBundled(m.id),
            );
            y += ROW;
        }
        y = self.local_rows(art, ui, x, y, w, Content::Mods, t);
        if self.bundled.is_empty() && !self.local.iter().any(|l| l.kind == Content::Mods) {
            y = empty_note(ui, x, y, w, "No mods in this instance yet.");
        }
        y
    }

    fn home_packs(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = list_head(
            art,
            ui,
            t,
            x,
            y,
            w,
            "One shader pack at a time; resource packs stack, the last on top.",
            &[
                ("Find packs", "search-01", Action::Page(Page::Packs)),
                ("Add pack", "plus-sign", Action::Import(Content::Packs)),
                ("Add shader", "plus-sign", Action::Import(Content::Shaders)),
            ],
        );
        y = card_heading(ui, x, y, "Resource packs");
        let before = y;
        y = self.local_rows(art, ui, x, y, w, Content::Packs, t);
        if y == before {
            y = empty_note(ui, x, y, w, "None yet.");
        }
        y = card_heading(ui, x, y + 8., "Shader packs");
        let before = y;
        y = self.local_rows(art, ui, x, y, w, Content::Shaders, t);
        if y == before {
            y = empty_note(ui, x, y, w, "None yet.");
        }
        y
    }

    fn home_worlds(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = list_head(
            art,
            ui,
            t,
            x,
            y,
            w,
            "This instance's single-player worlds.",
            &[
                ("Folder", "folder-01", Action::WorldsFolder),
                ("Backups", "download-04", Action::BackupsFolder),
            ],
        );
        if self.worlds.is_empty() {
            y = empty_note(
                ui,
                x,
                y,
                w,
                "No worlds yet. Press Play and make one, or copy a world folder in with Folder.",
            );
        }
        for (i, world) in self.worlds.iter().enumerate() {
            list_row(
                art,
                ui,
                x,
                y,
                w,
                124.,
                world.icon.as_ref(),
                "earth",
                &world.name,
                &world.note,
            );
            art.button(
                ui,
                t,
                [x + w - 118., y + 11., 70., 38.],
                "Play",
                Action::PlayWorld(i),
                Kind::Secondary,
            );
            art.icon_button(
                ui,
                t,
                [x + w - 44., y + 8., 44., 44.],
                "more-vertical",
                Action::WorldMenu(i),
                false,
            );
            y += ROW;
        }
        y
    }

    fn home_servers(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = list_head(
            art,
            ui,
            t,
            x,
            y,
            w,
            "The game's own server list. Join starts the game straight into one.",
            &[("Add server", "plus-sign", Action::AddServer)],
        );
        if self.servers.is_empty() {
            y = empty_note(ui, x, y, w, "No servers yet.");
        }
        for (i, s) in self.servers.iter().enumerate() {
            list_row(
                art,
                ui,
                x,
                y,
                w,
                124.,
                s.icon.as_ref(),
                "globe-02",
                &s.name,
                &s.address,
            );
            art.button(
                ui,
                t,
                [x + w - 118., y + 11., 70., 38.],
                "Join",
                Action::JoinServer(i),
                Kind::Secondary,
            );
            art.icon_button(
                ui,
                t,
                [x + w - 44., y + 8., 44., 44.],
                "delete-02",
                Action::RemoveServer(i),
                false,
            );
            y += ROW;
        }
        y
    }

    fn home_shots(&self, _art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        if self.shots.is_empty() {
            return empty_note(ui, x, y, w, "F2 in the game saves a screenshot here.");
        }
        let cols = if w >= 560. { 3 } else { 2 };
        let gap = 8.;
        let tw = (w - gap * (cols - 1) as f32) / cols as f32;
        let th = tw * 9. / 16.;
        for (i, s) in self.shots.iter().enumerate() {
            let (c, r) = ((i % cols) as f32, (i / cols) as f32);
            let rect = [x + c * (tw + gap), y + r * (th + gap), tw, th];
            shot_tile(ui, s, rect);
            t.push((rect, Action::OpenShot(i)));
        }
        y + self.shots.len().div_ceil(cols) as f32 * (th + gap)
    }

    fn home_console(&self, art: &Art, ui: &mut Ui, b: [f32; 4], t: &mut Targets<Action>) -> f32 {
        let [x, y, w, _] = b;
        let mut y = list_head(
            art,
            ui,
            t,
            x,
            y,
            w,
            "This run's log, from Play to now.",
            &[
                ("Copy", "copy-01", Action::CopyLog),
                ("Export report", "upload-04", Action::ExportDiagnostics),
            ],
        );
        ui.rounded(x, y, w, 340., radius::ROW, tone(tokens::SCRIM, 0.55));
        let line = 17.;
        let rows = 19;
        let start = self.log.len().saturating_sub(rows);
        if self.log.is_empty() {
            ui.text(
                Face::Mono,
                12.,
                x + 12.,
                y + 10.,
                white(ink::QUIET),
                "Nothing yet.",
            );
        }
        for (i, l) in self.log[start..].iter().enumerate() {
            let c = if l.contains("[ERROR]") {
                hex(tokens::DANGER, 255)
            } else if l.contains("[WARN]") {
                hex(tokens::WARN, 255)
            } else {
                white(ink::SOFT)
            };
            fitted(
                ui,
                Face::Mono,
                12.,
                x + 12.,
                y + 8. + i as f32 * line,
                w - 24.,
                c,
                l,
            );
        }
        y += 348.;
        y
    }

    fn browser(&self, art: &Art, ui: &mut Ui, a: [f32; 4], y: f32, t: &mut Targets<Action>) -> f32 {
        let [x, _, w, _] = a;
        let mut y = page_head(
            ui,
            a,
            y + 8.,
            "Mods",
            "Horizon's own registry and Modrinth. Files are checked before they go in.",
        );
        let strip = [x, y, w, 52.];
        let start = ui.batch.vertices.len();
        let mut tx = x + 8.;
        let mut ty = y + 8.;
        for (page, label) in [
            (Page::Mods, "Mods"),
            (Page::Packs, "Resource packs"),
            (Page::Shaders, "Shader packs"),
            (Page::DataPacks, "Data packs"),
        ] {
            let width = ui.measure(Face::Medium, 14., label) + 24.;
            if tx + width > x + w - 8. {
                tx = x + 8.;
                ty += 40.;
            }
            tx += art.tab(
                ui,
                t,
                tx,
                ty,
                36.,
                None,
                label,
                None,
                self.page == page,
                Action::Page(page),
            ) + 4.;
        }
        let strip_h = ty + 44. - y;
        under(ui, start, |ui| {
            art.plate(
                ui,
                [strip[0], strip[1], w, strip_h],
                radius::LARGE,
                Plate::Glass,
            );
        });
        y += strip_h + 10.;
        let panel_top = y;
        let start = ui.batch.vertices.len();
        let (px, pw) = (x + 12., w - 24.);
        y += 12.;
        let is_url = self.page == Page::Mods && self.query.trim().starts_with("https://");
        let sources = self.source.is_some();
        let narrow = pw < 560.;
        let source_w = if sources { 196. } else { 0. };
        let search_w = pw - if narrow { 0. } else { source_w + 8. } - 100.;
        art.field(
            ui,
            t,
            [px, y, search_w, 46.],
            Some("search-01"),
            &self.query,
            if self.page == Page::Mods {
                "Search, or paste an https .hmod link"
            } else {
                "Search"
            },
            self.editing == Some("search"),
            Action::Field("search"),
        );
        art.button(
            ui,
            t,
            [px + search_w + 8., y + 3., 92., 40.],
            if is_url { "Import" } else { "Search" },
            Action::Search,
            Kind::Primary,
        );
        if let Some(source) = self.source {
            use crate::shader_catalog::Origin;
            let pick = |o: Origin| {
                if self.page == Page::Mods {
                    Action::ModSource(o)
                } else {
                    Action::ShaderSource(o)
                }
            };
            let sx = if narrow { px } else { px + pw - source_w };
            let sy = if narrow { y + 54. } else { y };
            art.segments(
                ui,
                t,
                [sx, sy, if narrow { pw } else { source_w }, 46.],
                &[
                    ("Horizon", pick(Origin::Horizon), source == Origin::Horizon),
                    (
                        "Modrinth",
                        pick(Origin::Modrinth),
                        source == Origin::Modrinth,
                    ),
                ],
            );
            if narrow {
                y += 54.;
            }
        }
        y += 54.;
        if self.page == Page::DataPacks {
            let label = match &self.data_world {
                Some(world) => format!("Into world: {world}"),
                None => "Make a world first: data packs go into one world".to_owned(),
            };
            fitted(
                ui,
                Face::Regular,
                13.,
                px,
                y,
                pw - 120.,
                white(ink::BODY),
                &label,
            );
            if self.data_world.is_some() {
                art.button(
                    ui,
                    t,
                    [px + pw - 110., y - 8., 110., 36.],
                    "Change",
                    Action::PickDataPackWorld(0),
                    Kind::Secondary,
                );
            }
            y += 32.;
        }
        if !self.search_hint.is_empty() {
            y += paragraph(
                ui,
                Face::Regular,
                13.,
                px,
                y,
                pw,
                white(ink::QUIET),
                &self.search_hint,
            ) + 6.;
        }
        if self.page != Page::DataPacks {
            let kind = match self.page {
                Page::Mods => Content::Mods,
                Page::Shaders => Content::Shaders,
                _ => Content::Packs,
            };
            art.icon_label(
                ui,
                t,
                [px, y, (pw - 8.) / 2., 40.],
                Some("folder-01"),
                "Open folder",
                Action::OpenFolder(kind),
                Kind::Secondary,
            );
            art.icon_label(
                ui,
                t,
                [px + (pw + 8.) / 2., y, (pw - 8.) / 2., 40.],
                Some("upload-04"),
                "Import file",
                Action::Import(kind),
                Kind::Secondary,
            );
            y += 50.;
        }
        if self.page == Page::Mods {
            for m in &self.bundled {
                list_row(art, ui, px, y, pw, 60., None, "puzzle", &m.title, &m.note);
                art.toggle(
                    ui,
                    t,
                    [px + pw - 56., y + 8., 56., 44.],
                    m.enabled,
                    Action::ToggleBundled(m.id),
                );
                y += ROW;
            }
        }
        for p in &self.results {
            list_row(
                art,
                ui,
                px,
                y,
                pw,
                120.,
                None,
                "package-01",
                &p.title,
                &p.note,
            );
            if let Some(state) = &p.state {
                let width = ui.measure(Face::Medium, 13., state);
                ui.text(
                    Face::Medium,
                    13.,
                    px + pw - width.min(116.),
                    y + 21.,
                    white(ink::MUTED),
                    state,
                );
            } else if let Some(label) = &p.install {
                art.button(
                    ui,
                    t,
                    [px + pw - 100., y + 11., 100., 38.],
                    label,
                    Action::Install(p.index),
                    Kind::Secondary,
                );
            }
            y += ROW;
        }
        y += 8.;
        if self.offset > 0 {
            art.button(
                ui,
                t,
                [px, y, 110., 40.],
                "Previous",
                Action::Previous,
                Kind::Secondary,
            );
        }
        if self.results.len() >= 12 {
            art.button(
                ui,
                t,
                [px + pw - 110., y, 110., 40.],
                "Next",
                Action::Next,
                Kind::Secondary,
            );
        }
        y += 52.;
        under(ui, start, |ui| {
            art.plate(
                ui,
                [x, panel_top, w, y - panel_top],
                radius::PANEL,
                Plate::Glass,
            );
        });
        y
    }

    fn versions(
        &self,
        art: &Art,
        ui: &mut Ui,
        a: [f32; 4],
        y: f32,
        t: &mut Targets<Action>,
    ) -> f32 {
        let [x, _, w, _] = a;
        let mut y = page_head(
            ui,
            a,
            y + 8.,
            "Versions",
            "One app build plays one game version. A newer build installs over this one and keeps your worlds.",
        );
        let top = y;
        let start = ui.batch.vertices.len();
        let (px, pw) = (x + 12., w - 24.);
        y += 12.;
        y = card_heading(ui, px, y, "On this device");
        list_row(
            art,
            ui,
            px,
            y,
            pw,
            0.,
            None,
            "package-01",
            &format!("Horizon Android {}", self.version),
            &format!("Minecraft {}  ·  {}", self.minecraft, self.settings.device),
        );
        y += ROW + 8.;
        rule(ui, px, y, pw);
        y += 10.;
        let head_y = y;
        y = card_heading(ui, px, y, "Published builds");
        art.icon_button(
            ui,
            t,
            [px + pw - 40., head_y - 8., 40., 40.],
            "refresh",
            Action::RefreshReleases,
            false,
        );
        if self.releases.is_empty() {
            y = empty_note(ui, px, y, pw - 40., &self.releases_note);
        }
        for (i, r) in self.releases.iter().enumerate() {
            list_row(
                art,
                ui,
                px,
                y,
                pw,
                100.,
                None,
                "download-04",
                &r.name,
                &r.note,
            );
            if r.current {
                ui.text(
                    Face::Medium,
                    13.,
                    px + pw - 60.,
                    y + 21.,
                    hex(tokens::LIVE, 255),
                    "In use",
                );
            } else {
                art.button(
                    ui,
                    t,
                    [px + pw - 90., y + 11., 90., 38.],
                    "Get APK",
                    Action::OpenRelease(i),
                    Kind::Secondary,
                );
            }
            y += ROW;
        }
        y += 12.;
        under(ui, start, |ui| {
            art.plate(ui, [x, top, w, y - top], radius::PANEL, Plate::Glass);
        });
        y
    }

    fn notes(&self, art: &Art, ui: &mut Ui, a: [f32; 4], y: f32, t: &mut Targets<Action>) -> f32 {
        let [x, _, w, _] = a;
        let mut y = page_head(
            ui,
            a,
            y + 8.,
            "Patch notes",
            "Each release's notes, newest first.",
        );
        let top = y;
        let start = ui.batch.vertices.len();
        let (px, pw) = (x + 16., w - 32.);
        y += 12.;
        if self.releases.is_empty() {
            y = empty_note(ui, px, y, pw, &self.releases_note);
        }
        for (i, r) in self.releases.iter().enumerate() {
            let row_top = y;
            art.icon(
                ui,
                if r.open {
                    "arrow-down-01"
                } else {
                    "arrow-right-01"
                },
                px,
                y + 12.,
                18.,
                white(ink::MUTED),
            );
            fitted(
                ui,
                Face::SemiBold,
                15.,
                px + 26.,
                y + 3.,
                pw - 26.,
                white(ink::FULL),
                &r.name,
            );
            fitted(
                ui,
                Face::Regular,
                12.5,
                px + 26.,
                y + 25.,
                pw - 26.,
                white(ink::QUIET),
                &r.note,
            );
            t.push(([x, row_top, w, 50.], Action::OpenNote(i)));
            y += 50.;
            if r.open {
                y += paragraph(
                    ui,
                    Face::Regular,
                    14.,
                    px + 26.,
                    y,
                    pw - 26.,
                    white(ink::BODY),
                    &r.body,
                ) + 10.;
            }
            rule(ui, px, y, pw);
            y += 4.;
        }
        y += 8.;
        under(ui, start, |ui| {
            art.plate(ui, [x, top, w, y - top], radius::PANEL, Plate::Glass);
        });
        y
    }

    fn settings(
        &self,
        art: &Art,
        ui: &mut Ui,
        a: [f32; 4],
        y: f32,
        t: &mut Targets<Action>,
    ) -> f32 {
        let [x, _, w, _] = a;
        let s = &self.settings;
        let mut y = page_head(ui, a, y + 8., "Settings", "");
        art.segments(
            ui,
            t,
            [x, y, w.min(520.), 46.],
            &[
                (
                    "Launcher",
                    Action::SettingsTab(SettingsTab::Launcher),
                    s.tab == SettingsTab::Launcher,
                ),
                (
                    "Game",
                    Action::SettingsTab(SettingsTab::Game),
                    s.tab == SettingsTab::Game,
                ),
                (
                    "Screen",
                    Action::SettingsTab(SettingsTab::Screen),
                    s.tab == SettingsTab::Screen,
                ),
                (
                    "Reports",
                    Action::SettingsTab(SettingsTab::Reports),
                    s.tab == SettingsTab::Reports,
                ),
            ],
        );
        y += 58.;
        let top = y;
        let start = ui.batch.vertices.len();
        let (px, pw) = (x + 16., w - 32.);
        y += 6.;
        match s.tab {
            SettingsTab::Launcher => {
                y = setting(
                    ui,
                    px,
                    y,
                    pw,
                    "Accent",
                    "The main colour, the sky's hue with it.",
                    244.,
                    |ui, r| {
                        for (i, (_, c)) in ACCENTS.iter().enumerate() {
                            let rect = [r[0] + i as f32 * 35., r[1] + 8., 32., 32.];
                            ui.rounded(rect[0], rect[1], 32., 32., 16., hex(*c, 255));
                            if s.accent == i {
                                art.icon(
                                    ui,
                                    "tick-02",
                                    rect[0] + 7.,
                                    rect[1] + 7.,
                                    18.,
                                    white(ink::FULL),
                                );
                            }
                            t.push((rect, Action::Accent(i)));
                        }
                    },
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Plain plates",
                    "Solid plates without the scene: lighter on the GPU and battery.",
                    s.plain,
                    Action::PlainGlass,
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Still sky",
                    "Hold the stars and fireflies.",
                    s.still,
                    Action::StillScene,
                );
                y = text_setting(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Proxy",
                    "http://host:port or socks5://host:port for every download. Empty: none.",
                    &s.proxy,
                    "No proxy",
                    self.editing == Some("proxy"),
                    Action::Field("proxy"),
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Files",
                    "Worlds, mods, packs and reports in the system's Files app.",
                    "Open",
                    Action::GameFiles,
                );
            }
            SettingsTab::Game => {
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Rendering",
                    "Applies on the next Play.",
                    &s.renderer,
                    Action::Renderer,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Client resolution",
                    "Scales the whole picture, menus too.",
                    &s.resolution,
                    Action::RenderScale,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Chunk worker limit",
                    "Ceiling for generation and meshing.",
                    &s.workers,
                    Action::WorkerLimit,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Reusable RAM cache",
                    "Decoded sounds and empty mesh buffers.",
                    &s.cache,
                    Action::CacheBudget,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Input",
                    "Touch, or a physical keyboard and mouse.",
                    &s.input,
                    Action::InputHost,
                );
                y = text_setting(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Game arguments",
                    "Extra arguments for the game, passed as they are. Applies on the next Play.",
                    &s.game_args,
                    "For example --server-thread",
                    self.editing == Some("game_args"),
                    Action::Field("game_args"),
                );
                y = text_setting(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Join at start",
                    "A server address the game connects to on Play.",
                    &s.server,
                    "Single player",
                    self.editing == Some("server"),
                    Action::Field("server"),
                );
            }
            SettingsTab::Screen => {
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Orientation",
                    "The launcher and the game.",
                    &s.orientation,
                    Action::Orientation,
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Immersive fullscreen",
                    "Hide the system bars.",
                    s.immersive,
                    Action::Immersive,
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Avoid camera cutout",
                    "Keep the picture clear of the camera.",
                    s.cutout,
                    Action::Cutout,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Aspect ratio",
                    "Fits the entire picture without stretching.",
                    &s.aspect,
                    Action::Aspect,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Display refresh rate",
                    "A display preference, not an FPS cap.",
                    &s.display,
                    Action::DisplayHz,
                );
            }
            SettingsTab::Reports => {
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Performance hints",
                    "ADPF: frame-work hints to Android, on supported devices.",
                    s.hints,
                    Action::PerformanceHints,
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "GPU profiling",
                    "Records GPU timings in the report. Has overhead.",
                    s.gpu_profile,
                    Action::GpuProfile,
                );
                y = switch(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Performance recording",
                    "Up to 30 minutes from the next Play.",
                    s.recording,
                    Action::RecordPerformance,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Diagnostics",
                    "The last run's summary and error.",
                    "Show",
                    Action::Logs,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Diagnostic report",
                    "One ZIP in Reports. Nothing is uploaded.",
                    "Export",
                    Action::ExportDiagnostics,
                );
                y = pick(
                    art,
                    ui,
                    t,
                    px,
                    y,
                    pw,
                    "Raw performance data",
                    "Timing files, in Reports.",
                    "Open",
                    Action::ExportPerformance,
                );
            }
        }
        y += 6.;
        under(ui, start, |ui| {
            art.plate(ui, [x, top, w, y - top], radius::PANEL, Plate::Glass);
        });
        y
    }
}

/// The accents Settings offers: the desktop's, by name.
pub const ACCENTS: [(&str, u32); 7] = [
    ("Violet", tokens::ACCENT),
    ("Blue", 0x3b82f6),
    ("Teal", 0x14b8a6),
    ("Green", 0x22c55e),
    ("Amber", 0xf59e0b),
    ("Red", 0xef4444),
    ("Pink", 0xec4899),
];

fn shot_tile(ui: &mut Ui, s: &ShotRow, rect: [f32; 4]) {
    // Only what's on screen goes to the atlas: it holds a few dozen thumbnails, not hundreds.
    if rect[1] + rect[3] < 0. || rect[1] > ui.h {
        return;
    }
    ui.rounded(
        rect[0],
        rect[1],
        rect[2],
        rect[3],
        radius::ROW,
        white(fill::FIELD),
    );
    if let Some((key, pixels)) = &s.thumb {
        picture(ui, *key, pixels, rect);
    }
}

/// A row of a settings card: its name and note on the left, `control` drawn at its right `room`.
#[allow(clippy::too_many_arguments)]
fn setting(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    note: &str,
    room: f32,
    control: impl FnOnce(&mut Ui, [f32; 4]),
) -> f32 {
    let stacked = w < 420. + room * 0.4;
    let text_w = if stacked { w } else { w - room - 16. };
    ui.text(Face::Medium, 14., x, y + 10., white(ink::FULL), name);
    let h = paragraph(
        ui,
        Face::Regular,
        13.,
        x,
        y + 32.,
        text_w,
        white(ink::QUIET),
        note,
    );
    let text_bottom = y + 32. + h + 10.;
    let bottom = if stacked {
        control(ui, [x, text_bottom - 4., w, 48.]);
        text_bottom + 52.
    } else {
        let bottom = text_bottom.max(y + 64.);
        control(ui, [x + w - room, y + (bottom - y - 48.) / 2., room, 48.]);
        bottom
    };
    rule(ui, x, bottom, w);
    bottom + 1.
}

#[allow(clippy::too_many_arguments)]
fn switch(
    art: &Art,
    ui: &mut Ui,
    t: &mut Targets<Action>,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    note: &str,
    on: bool,
    action: Action,
) -> f32 {
    // The whole row is the target; the knob sits at its end.
    let text_w = w - 64.;
    ui.text(Face::Medium, 14., x, y + 10., white(ink::FULL), name);
    let h = paragraph(
        ui,
        Face::Regular,
        13.,
        x,
        y + 32.,
        text_w,
        white(ink::QUIET),
        note,
    );
    let bottom = (y + 32. + h + 10.).max(y + 64.);
    art.toggle(ui, t, [x, y, w, bottom - y], on, action);
    rule(ui, x, bottom, w);
    bottom + 1.
}

#[allow(clippy::too_many_arguments)]
fn pick(
    art: &Art,
    ui: &mut Ui,
    t: &mut Targets<Action>,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    note: &str,
    value: &str,
    action: Action,
) -> f32 {
    setting(ui, x, y, w, name, note, 230., |ui, r| {
        art.button(
            ui,
            t,
            [r[0] + (r[2] - 230.).max(0.), r[1] + 4., r[2].min(230.), 40.],
            value,
            action,
            Kind::Secondary,
        );
    })
}

#[allow(clippy::too_many_arguments)]
fn text_setting(
    art: &Art,
    ui: &mut Ui,
    t: &mut Targets<Action>,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    note: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
    action: Action,
) -> f32 {
    setting(ui, x, y, w, name, note, 280., |ui, r| {
        art.field(
            ui,
            t,
            [r[0], r[1] + 2., r[2], 44.],
            None,
            value,
            placeholder,
            focused,
            action,
        );
    })
}

/// A list's note and its buttons (`list_head`). Buttons drop under the note when it's narrow.
#[allow(clippy::too_many_arguments)]
fn list_head(
    art: &Art,
    ui: &mut Ui,
    t: &mut Targets<Action>,
    x: f32,
    y: f32,
    w: f32,
    note: &str,
    buttons: &[(&str, &str, Action)],
) -> f32 {
    let widths: Vec<f32> = buttons
        .iter()
        .map(|(l, _, _)| ui.measure(Face::Medium, 14., l) + 54.)
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + 8. * (buttons.len() as f32 - 1.);
    let beside = w - total >= 240.;
    let room = if beside { w - total - 16. } else { w };
    let h = paragraph(
        ui,
        Face::Regular,
        13.,
        x,
        y + 4.,
        room,
        white(ink::QUIET),
        note,
    );
    let by = if beside { y } else { y + h + 12. };
    let mut bx = if beside { x + w - total } else { x };
    let scale = if beside { 1. } else { (w / total).min(1.) };
    for ((label, icon, action), bw) in buttons.iter().zip(&widths) {
        let bw = bw * scale;
        art.icon_label(
            ui,
            t,
            [bx, by, bw - if scale < 1. { 8. } else { 0. }, 40.],
            Some(*icon),
            label,
            *action,
            Kind::Secondary,
        );
        bx += bw + if scale < 1. { 0. } else { 8. };
    }
    if beside {
        y + (h + 4.).max(40.) + 12.
    } else {
        by + 52.
    }
}

pub fn scrim(ui: &mut Ui) {
    let (w, h) = (ui.w, ui.h);
    ui.rounded(0., 0., w, h, 0., hex(tokens::SCRIM, 150));
}

/// One choice of a [`Menu`].
#[derive(Clone)]
pub struct MenuRow {
    pub icon: &'static str,
    pub label: String,
    pub action: Action,
    pub danger: bool,
}

/// A list of what can be done to one thing, over the page.
#[derive(Clone)]
pub struct Menu {
    pub title: String,
    pub note: String,
    pub rows: Vec<MenuRow>,
}
impl Menu {
    pub fn draw(&self, art: &Art, ui: &mut Ui, out: &mut Out) {
        out.targets.clear();
        out.list.clear();
        out.scroll = None;
        scrim(ui);
        out.targets.push(([0., 0., ui.w, ui.h], Action::Dismiss));
        let w = (ui.w - 2. * EDGE).min(420.);
        let note_h = wrapped(ui, Face::Regular, 13., &self.note, w - 40.).len() as f32 * 20.;
        let h = (56. + note_h + self.rows.len() as f32 * 48. + 16.).min(ui.h - 2. * EDGE);
        let x = (ui.w - w) / 2.;
        let y = (ui.h - h) / 2.;
        art.plate(ui, [x, y, w, h], radius::POPOVER, Plate::Float);
        fitted(
            ui,
            Face::SemiBold,
            16.,
            x + 20.,
            y + 16.,
            w - 40.,
            white(ink::FULL),
            &self.title,
        );
        let mut row = y + 46.;
        if !self.note.is_empty() {
            row += paragraph(
                ui,
                Face::Regular,
                13.,
                x + 20.,
                row,
                w - 40.,
                white(ink::QUIET),
                &self.note,
            ) + 4.;
        }
        for r in &self.rows {
            let c = if r.danger {
                hex(tokens::DANGER, 255)
            } else {
                white(ink::STRONG)
            };
            art.icon(ui, r.icon, x + 20., row + 13., 20., c);
            ui.text(Face::Regular, 15., x + 52., row + 13., c, &r.label);
            out.targets.push(([x + 8., row, w - 16., 46.], r.action));
            row += 48.;
        }
    }
}

/// A question: its buttons in a row at the bottom.
pub fn dialog(
    art: &Art,
    ui: &mut Ui,
    out: &mut Out,
    title: &str,
    note: &str,
    buttons: &[(&str, Action, Kind)],
) {
    out.targets.clear();
    out.list.clear();
    out.scroll = None;
    scrim(ui);
    let w = (ui.w - 2. * EDGE).min(440.);
    let lines = wrapped(ui, Face::Regular, 14., note, w - 48.).len() as f32;
    let h =
        32. * if ui.measure(Face::Display, 26., title) > w - 48. {
            2.
        } else {
            1.
        } + lines * 21.
            + 112.;
    let (x, y) = ((ui.w - w) / 2., (ui.h - h) / 2.);
    art.plate(ui, [x, y, w, h], radius::MODAL, Plate::Float);
    head(ui, x + 24., y + 20., w - 48., title, note);
    let each = (w - 48. - 8. * (buttons.len() as f32 - 1.)) / buttons.len() as f32;
    for (i, (label, action, kind)) in buttons.iter().enumerate() {
        art.button(
            ui,
            &mut out.targets,
            [x + 24. + i as f32 * (each + 8.), y + h - 62., each, 42.],
            label,
            *action,
            *kind,
        );
    }
}

/// A form: fields with labels, and its buttons.
#[derive(Clone)]
pub struct Form {
    pub title: String,
    pub note: String,
    /// `(label, field, value, placeholder)`.
    pub fields: Vec<(String, &'static str, String, String)>,
    pub hues: Option<usize>,
    pub buttons: Vec<(String, Action, Kind)>,
}

/// The tile colours an instance can take.
pub const HUES: [u32; 7] = [
    0x6b4bff, 0x2563eb, 0x0f766e, 0x15803d, 0xb45309, 0xb91c1c, 0xbe185d,
];

impl Form {
    pub fn draw(&self, art: &Art, ui: &mut Ui, out: &mut Out, editing: Option<&str>) {
        out.targets.clear();
        out.list.clear();
        out.scroll = None;
        scrim(ui);
        let w = (ui.w - 2. * EDGE).min(460.);
        let lines = if self.note.is_empty() {
            0.
        } else {
            wrapped(ui, Face::Regular, 14., &self.note, w - 48.).len() as f32
        };
        let h = 68.
            + lines * 21.
            + self.fields.len() as f32 * 78.
            + if self.hues.is_some() { 52. } else { 0. }
            + 74.;
        // The keyboard takes the bottom of the screen: stay on what's left.
        let (x, y) = ((ui.w - w) / 2., ((ui.h - h) / 2.).max(EDGE));
        art.plate(ui, [x, y, w, h], radius::MODAL, Plate::Float);
        let mut row = head(ui, x + 24., y + 20., w - 48., &self.title, &self.note) + 6.;
        for (label, field, value, placeholder) in &self.fields {
            ui.text(Face::Medium, 13., x + 24., row, white(ink::BODY), label);
            art.field(
                ui,
                &mut out.targets,
                [x + 24., row + 22., w - 48., 46.],
                None,
                value,
                placeholder,
                editing == Some(*field),
                Action::Field(field),
            );
            row += 78.;
        }
        if let Some(chosen) = self.hues {
            for (i, c) in HUES.iter().enumerate() {
                let r = [x + 24. + i as f32 * 42., row + 4., 36., 36.];
                ui.rounded(r[0], r[1], 36., 36., radius::CONTROL, hex(*c, 255));
                if i == chosen {
                    art.icon(ui, "tick-02", r[0] + 8., r[1] + 8., 20., white(ink::FULL));
                }
                out.targets.push((r, Action::InstanceHue(i)));
            }
        }
        let each = (w - 48. - 8. * (self.buttons.len() as f32 - 1.)) / self.buttons.len() as f32;
        for (i, (label, action, kind)) in self.buttons.iter().enumerate() {
            art.button(
                ui,
                &mut out.targets,
                [x + 24. + i as f32 * (each + 8.), y + h - 62., each, 42.],
                label,
                *action,
                *kind,
            );
        }
    }
}

/// A screenshot over the page, with what can be done to it.
pub fn shot_viewer(
    art: &Art,
    ui: &mut Ui,
    out: &mut Out,
    name: &str,
    image: Option<&(u64, Pixels)>,
) {
    out.targets.clear();
    out.list.clear();
    out.scroll = None;
    let (w, h) = (ui.w, ui.h);
    ui.rounded(0., 0., w, h, 0., hex(tokens::SCRIM, 235));
    if let Some((key, pixels)) = image {
        let room = [w - 2. * EDGE, h - 130.];
        let s = (room[0] / pixels.0 as f32).min(room[1] / pixels.1 as f32);
        let (iw, ih) = (pixels.0 as f32 * s, pixels.1 as f32 * s);
        picture(
            ui,
            *key,
            pixels,
            [(w - iw) / 2., (h - 72. - ih) / 2., iw, ih],
        );
    }
    let y = h - 60.;
    fitted(
        ui,
        Face::Regular,
        13.,
        EDGE + 4.,
        y - 26.,
        w - 2. * EDGE,
        white(ink::MUTED),
        name,
    );
    let buttons: [(&str, &str, Action); 5] = [
        ("Previous", "", Action::ShotStep(false)),
        ("Next", "", Action::ShotStep(true)),
        ("Save", "download-04", Action::SaveShot),
        ("Delete", "delete-02", Action::DeleteShot),
        ("Close", "cancel-01", Action::CloseShot),
    ];
    let each = (w - 2. * EDGE - 32.) / 5.;
    for (i, (label, icon, action)) in buttons.into_iter().enumerate() {
        art.icon_label(
            ui,
            &mut out.targets,
            [EDGE + i as f32 * (each + 8.), y, each, 44.],
            (w >= 520. && !icon.is_empty()).then_some(icon),
            label,
            action,
            if action == Action::DeleteShot {
                Kind::Danger
            } else {
                Kind::Secondary
            },
        );
    }
}
