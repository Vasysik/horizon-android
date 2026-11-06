//! Draws the Android launcher's screens on a computer: the same `launcher_view.rs` and
//! `launcher_glass.rs` the APK uses, over the same `HudBatch`, rasterised in software to PNGs,
//! at a Fold's inner, cover and portrait sizes.
//!
//! `cargo run -p horizon-client --example android_launcher_preview -- <out dir>`
#![allow(dead_code)]

/// Stand-ins for the Android-only types an [`Action`](launcher_view::Action) can carry.
mod shader_catalog {
    #[derive(Clone, Copy, PartialEq)]
    pub enum Origin {
        Horizon,
        Modrinth,
    }
}
mod display {
    #[derive(Clone, Copy, PartialEq)]
    pub struct RenderScale;
    #[derive(Clone, Copy, PartialEq)]
    pub struct Aspect;
}
mod catalog {
    #[derive(Clone, Copy, PartialEq)]
    pub struct InputHost;
    #[derive(Clone, Copy, PartialEq)]
    pub struct Orientation;
    #[derive(Clone, Copy, PartialEq)]
    pub struct RendererMode;
}
#[path = "../src/android/launcher_data.rs"]
mod launcher_data;
#[path = "../src/android/launcher_glass.rs"]
mod launcher_glass;
#[path = "../src/android/launcher_scene.rs"]
mod launcher_scene;
#[path = "../src/android/launcher_tokens.rs"]
mod launcher_tokens;
#[path = "../src/android/launcher_view.rs"]
mod launcher_view;
#[path = "../../../launcher/src/worlds.rs"]
mod worlds;
#[path = "../src/android/launcher_servers.rs"]
mod server_list;
#[path = "../../../launcher/src/screenshots.rs"]
mod screenshots;
use launcher_data::packs;
#[path = "../src/android/launcher_datapacks.rs"]
mod datapacks;
mod desktop_seams {
    pub fn to_trash(path: &std::path::Path) -> Result<(), String> {
        crate::launcher_data::remove(path)
    }
}

use horizon_client::content_files::Kind as Content;
use horizon_client::hud::{HudAtlas, HudBatch};
use horizon_client::ui_theme::Ui;
use launcher_glass::{Art, Kind};
use launcher_view::*;
use std::path::Path;

fn picture(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> launcher_glass::Pixels {
    let mut p = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            p.extend_from_slice(&f(x, y));
        }
    }
    std::sync::Arc::new((w, h, p))
}

fn sample() -> View {
    let world = |name: &str, note: &str, key: u64| WorldRow {
        name: name.into(),
        note: note.into(),
        icon: Some((
            key,
            picture(64, 64, |x, y| {
                [(x * 3) as u8, 110 + (y * 2) as u8, 90 + ((x ^ y) * 2) as u8, 255]
            }),
        )),
    };
    let shot = |name: &str, key: u64| ShotRow {
        name: name.into(),
        thumb: Some((
            key,
            picture(240, 135, |x, y| {
                [
                    (x + key as u32 * 30) as u8,
                    (y * 2) as u8,
                    180u32.saturating_sub(y) as u8,
                    255,
                ]
            }),
        )),
    };
    View {
        version: "0.7.7-dev".into(),
        minecraft: "26.3".into(),
        page: Page::Play,
        home_tab: HomeTab::Overview,
        drawer: false,
        account_name: "Steve".into(),
        account_how: "Offline".into(),
        instances: vec![
            InstanceRow {
                id: 0,
                name: "Horizon".into(),
                note: "Ready · 26.3".into(),
                hue: HUES[0],
                selected: true,
            },
            InstanceRow {
                id: 1,
                name: "Survival with Ty".into(),
                note: "Played 12 h".into(),
                hue: HUES[2],
                selected: false,
            },
            InstanceRow {
                id: 2,
                name: "Creative tests".into(),
                note: "Never played".into(),
                hue: HUES[4],
                selected: false,
            },
        ],
        played: "Played 4 h 20 min".into(),
        game: Game {
            installed: true,
            ..Game::default()
        },
        status: String::new(),
        editing: None,
        friend: String::new(),
        friend_valid: false,
        bits: true,
        worlds: vec![
            world("New World", "Survival · 26.3 · 2 days ago · 84 MB", 11),
            world("Sky island", "Creative · 26.3 · today · 12 MB", 12),
        ],
        servers: vec![ServerRow {
            name: "Minecraft Server".into(),
            address: "play.example.net".into(),
            icon: None,
        }],
        local: vec![
            LocalRow {
                index: 0,
                kind: Content::Mods,
                title: "minimap.hmod".into(),
                enabled: true,
            },
            LocalRow {
                index: 1,
                kind: Content::Packs,
                title: "Faithful 32x.zip".into(),
                enabled: true,
            },
            LocalRow {
                index: 2,
                kind: Content::Shaders,
                title: "Sunrise.hshader".into(),
                enabled: false,
            },
        ],
        bundled: vec![BundledRow {
            id: "tweaks",
            title: "Tweaks".into(),
            note: "Bundled with the app".into(),
            enabled: true,
        }],
        shots: (0..7)
            .map(|i| shot(&format!("2026-11-0{}_12.00.00.png", i + 1), 20 + i))
            .collect(),
        log: (0..30)
            .map(|i| {
                format!(
                    "12:00:{i:02} [{}] horizon_client: line {i}",
                    if i % 9 == 3 {
                        "WARN"
                    } else if i % 13 == 5 {
                        "ERROR"
                    } else {
                        "INFO"
                    }
                )
            })
            .collect(),
        follow: true,
        query: String::new(),
        source: Some(shader_catalog::Origin::Horizon),
        search_hint: "Search Horizon mods or paste an HTTPS .hmod link".into(),
        results: (0..6)
            .map(|i| ProjectRow {
                index: i,
                title: format!("Project {i}"),
                note: "A short summary of what this project does for the game".into(),
                state: (i == 1).then(|| "Installed".into()),
                install: Some("Install".into()),
            })
            .collect(),
        offset: 0,
        data_world: Some("New World".into()),
        releases: vec![
            ReleaseRow {
                name: "Horizon 0.7.7".into(),
                note: "2026-11-04 · 112 MB".into(),
                body: "Fix inventory sneak and allow camera drag from the attack button.\nCull terrain hidden by opaque Nether fog.\nAndroid mouse ownership fixes.".into(),
                open: true,
                current: true,
            },
            ReleaseRow {
                name: "Horizon 0.7.6".into(),
                note: "2026-10-30 · 110 MB".into(),
                body: "Touch inventory modifiers.".into(),
                open: false,
                current: false,
            },
        ],
        releases_note: "No releases read yet.".into(),
        settings: SettingsView {
            renderer: "Vulkan / compatibility".into(),
            resolution: "100%".into(),
            workers: "No additional limit".into(),
            cache: "Engine default (80 MiB)".into(),
            orientation: "Landscape (sensor)".into(),
            input: "Touch".into(),
            aspect: "Native".into(),
            display: "System default".into(),
            device: "arm64-v8a".into(),
            ..SettingsView::default()
        },
    }
}

/// Software rasteriser for the HUD's triangles: axis-aligned quads, nearest sampling, source-over.
fn rasterise(atlas: &HudAtlas, batch: &HudBatch, pixels: &mut Vec<u8>, sheet: &mut Vec<u8>, w: u32, h: u32) {
    let (aw, ah) = (atlas.width as usize, atlas.height as usize);
    for p in &batch.patches {
        for row in 0..p.height as usize {
            let at = ((p.y as usize + row) * aw + p.x as usize) * 4;
            let from = row * p.width as usize * 4;
            sheet[at..at + p.width as usize * 4]
                .copy_from_slice(&p.rgba[from..from + p.width as usize * 4]);
        }
    }
    let s = batch.scale;
    for quad in batch.vertices.chunks_exact(6) {
        let [a, b, c, d] = [quad[0], quad[1], quad[2], quad[5]];
        let xs = [a.pos[0], b.pos[0], c.pos[0], d.pos[0]].map(|v| v * s);
        let ys = [a.pos[1], b.pos[1], c.pos[1], d.pos[1]].map(|v| v * s);
        let (x0, x1) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
        let (y0, y1) = (ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max));
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        // Affine in x and y: the corner at the least x and y, and the other corners' uv.
        let at = |p: [f32; 2]| quad.iter().find(|v| (v.pos[0] * s - p[0]).abs() < 0.01 && (v.pos[1] * s - p[1]).abs() < 0.01);
        let (Some(tl), Some(tr), Some(bl), Some(br)) = (at([x0, y0]), at([x1, y0]), at([x0, y1]), at([x1, y1])) else {
            continue;
        };
        for y in (y0.floor() as i32).max(0)..(y1.ceil() as i32).min(h as i32) {
            let ty = ((y as f32 + 0.5 - y0) / (y1 - y0)).clamp(0., 1.);
            for x in (x0.floor() as i32).max(0)..(x1.ceil() as i32).min(w as i32) {
                let tx = ((x as f32 + 0.5 - x0) / (x1 - x0)).clamp(0., 1.);
                let lerp = |p: [f32; 2], q: [f32; 2], t: f32| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t];
                let uv = lerp(lerp(tl.uv, tr.uv, tx), lerp(bl.uv, br.uv, tx), ty);
                let (u, v) = (
                    ((uv[0] * aw as f32) as usize).min(aw - 1),
                    ((uv[1] * ah as f32) as usize).min(ah - 1),
                );
                let t = &sheet[(v * aw + u) * 4..][..4];
                let col = tl.color;
                // The vertex colour is ABGR.
                let m = [col & 255, col >> 8 & 255, col >> 16 & 255, col >> 24 & 255].map(|c| c as f32 / 255.);
                let alpha = t[3] as f32 / 255. * m[3];
                let at = (y as usize * w as usize + x as usize) * 4;
                for i in 0..3 {
                    let src = t[i] as f32 / 255. * m[i];
                    let dst = pixels[at + i] as f32 / 255.;
                    pixels[at + i] = ((src * alpha + dst * (1. - alpha)) * 255.).round() as u8;
                }
            }
        }
    }
}

fn save(path: &Path, w: u32, h: u32, rgba: &[u8]) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(rgba).unwrap();
}

struct Shot<'a> {
    name: &'a str,
    view: View,
    scroll: f32,
    overlay: Option<fn(&Art, &mut Ui, &mut Out, &View)>,
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "launcher-preview".into());
    std::fs::create_dir_all(&dir).unwrap();
    let atlas = HudAtlas::build(Path::new("/nonexistent/horizon-preview")).unwrap();
    let base = sample();
    let with = |f: &dyn Fn(&mut View)| {
        let mut v = base.clone();
        f(&mut v);
        v
    };
    let shots: Vec<Shot> = vec![
        Shot { name: "home", view: base.clone(), scroll: 0., overlay: None },
        Shot { name: "home-scrolled", view: base.clone(), scroll: 160., overlay: None },
        Shot { name: "home-install", view: with(&|v| {
            v.game = Game {
                installing: true,
                steps: [
                    ("Download the game's files", true, false),
                    ("Prepare resources", false, true),
                    ("Verify", false, false),
                ]
                .iter()
                .map(|(l, a, b)| ((*l).to_owned(), *a, *b))
                .collect(),
                overall: 0.45,
                detail: "812.4 / 1204.0 MiB - verified downloads".into(),
                stage: "Downloading".into(),
                ..Game::default()
            };
        }), scroll: 0., overlay: None },
        Shot { name: "worlds", view: with(&|v| v.home_tab = HomeTab::Worlds), scroll: 120., overlay: None },
        Shot { name: "multiplayer", view: with(&|v| v.home_tab = HomeTab::Multiplayer), scroll: 120., overlay: None },
        Shot { name: "packs", view: with(&|v| v.home_tab = HomeTab::Packs), scroll: 120., overlay: None },
        Shot { name: "instance-mods", view: with(&|v| v.home_tab = HomeTab::Mods), scroll: 120., overlay: None },
        Shot { name: "screenshots", view: with(&|v| v.home_tab = HomeTab::Screenshots), scroll: 120., overlay: None },
        Shot { name: "console", view: with(&|v| v.home_tab = HomeTab::Console), scroll: 120., overlay: None },
        Shot { name: "versions", view: with(&|v| v.page = Page::Versions), scroll: 0., overlay: None },
        Shot { name: "mods", view: with(&|v| v.page = Page::Mods), scroll: 0., overlay: None },
        Shot { name: "datapacks", view: with(&|v| { v.page = Page::DataPacks; v.source = None; }), scroll: 0., overlay: None },
        Shot { name: "notes", view: with(&|v| v.page = Page::Notes), scroll: 0., overlay: None },
        Shot { name: "settings", view: with(&|v| v.page = Page::Settings), scroll: 0., overlay: None },
        Shot { name: "settings-game", view: with(&|v| { v.page = Page::Settings; v.settings.tab = SettingsTab::Game; }), scroll: 0., overlay: None },
        Shot { name: "status", view: with(&|v| { v.status = "Backed up New World to backups/2026-11-06_12-00-00_New World.zip".into(); }), scroll: 0., overlay: None },
        Shot { name: "menu", view: base.clone(), scroll: 0., overlay: Some(|art, ui, out, _| {
            Menu {
                title: "New World".into(),
                note: "Survival · 84 MB".into(),
                rows: vec![
                    MenuRow { icon: "play", label: "Play".into(), action: Action::Dismiss, danger: false },
                    MenuRow { icon: "copy-01", label: "Copy".into(), action: Action::Dismiss, danger: false },
                    MenuRow { icon: "delete-02", label: "Delete".into(), action: Action::Dismiss, danger: true },
                ],
            }
            .draw(art, ui, out);
        }) },
        Shot { name: "dialog", view: base.clone(), scroll: 0., overlay: Some(|art, ui, out, _| {
            dialog(art, ui, out, "Delete New World?", "It's gone for good: Android has no trash here. Back it up first if you might want it.", &[
                ("Cancel", Action::Dismiss, Kind::Secondary),
                ("Delete", Action::Dismiss, Kind::Danger),
            ]);
        }) },
        Shot { name: "form", view: base.clone(), scroll: 0., overlay: Some(|art, ui, out, _| {
            Form {
                title: "New instance".into(),
                note: "Its own worlds, options and packs.".into(),
                fields: vec![("Name".into(), "instance_name", "Survival".into(), "A name".into())],
                hues: Some(1),
                buttons: vec![
                    ("Cancel".into(), Action::Dismiss, Kind::Secondary),
                    ("Create".into(), Action::SaveInstance, Kind::Primary),
                ],
            }
            .draw(art, ui, out, Some("instance_name"));
        }) },
        Shot { name: "shot", view: base.clone(), scroll: 0., overlay: Some(|art, ui, out, v| {
            shot_viewer(art, ui, out, &v.shots[0].name, v.shots[0].thumb.as_ref());
        }) },
        Shot { name: "drawer", view: with(&|v| v.drawer = true), scroll: 0., overlay: Some(|art, ui, out, v| {
            if !area(ui).wide { v.drawer(art, ui, out); }
        }) },
    ];
    let screens = [
        ("fold-inner", 2176u32, 1812u32),
        ("fold-cover", 2520, 1080),
        ("portrait", 1080, 2520),
    ];
    let only = std::env::args().nth(2);
    for (screen, w, h) in screens {
        horizon_client::ui_theme::reset_atlas();
        let mut art = Art::default();
        let mut sheet = atlas.pixels.clone();
        for shot in &shots {
            if only.as_ref().is_some_and(|o| !shot.name.contains(o.as_str())) {
                continue;
            }
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            for c in pixels.chunks_exact_mut(4) {
                c[3] = 255;
            }
            horizon_client::ui_theme::reset_atlas();
            sheet.clone_from(&atlas.pixels);
            let mut batch = HudBatch::new([w, h]);
            let mut ui = Ui::mobile(&atlas, &mut batch, (0., 0.));
            art.prepare(ui.w, ui.h, launcher_tokens::ACCENT);
            art.background(&mut ui);
            let mut out = Out::default();
            shot.view.draw(&art, &mut ui, shot.scroll, &mut out);
            if let Some(f) = shot.overlay {
                f(&art, &mut ui, &mut out, &shot.view);
            }
            for (r, _) in &out.targets {
                assert!(r[2] > 0. && r[3] > 0., "{} has an empty target", shot.name);
            }
            ui.finish();
            rasterise(&atlas, &batch, &mut pixels, &mut sheet, w, h);
            save(
                &Path::new(&dir).join(format!("{screen}-{}.png", shot.name)),
                w,
                h,
                &pixels,
            );
        }
        println!("{screen}: {} screens", shots.len());
    }
}
