//! The mods' buttons (`widgets`): on the game's screens, drawn as the screen's own buttons are
//! and pressed before the screen sees the click; on the windows of the screens with slots,
//! placed from the slots of the container or of the player's inventory as the screen open lays
//! them out; on the touch controls, where they are placed in the player's layout as the game's
//! own buttons (game/touch.rs); and the looks mods and resource packs give the game's own touch
//! buttons. A press is the mod's keybind of that id ([`Game::mod_bind`]).
use super::{Game, Screen, mods::rich_text};
use horizon_client::{
    hud::{HudBatch, rgba},
    screen_draw::{self, Draw, Measure},
    touch::{Button, Place, Shape},
};
use horizon_mods::{Anchor, ButtonShape as ModShape, ScreenButton, WindowButton, WindowPart};

/// A mod's button on the touch controls, as it is drawn.
#[derive(Clone, Debug)]
pub(super) struct ModFace {
    /// `<mod id>:<button id>`, its name in the layout.
    pub(super) key: String,
    pub(super) label: String,
    /// The atlas sprite drawn on it instead of the label.
    pub(super) icon: Option<String>,
    /// A sprite that is the whole button, and its pressed one (a custom button's look).
    pub(super) skin: Option<(String, String)>,
    pub(super) shape: Shape,
}

/// How one of the game's touch buttons looks: a mod's style, else a pack's sprite, else drawn.
pub(super) struct ControlFace {
    pub(super) label: String,
    /// A mod's icon, drawn on the plate.
    pub(super) icon: Option<String>,
    /// A resource pack's whole button: its sprite and its pressed sprite.
    pub(super) skin: Option<(String, Option<String>)>,
    pub(super) shape: Shape,
}

/// A keybind's button: 16 GUI pixels tall, as wide as its label, as vanilla's buttons.
const WIDE_HEIGHT: f32 = 16.0;

/// How far past its box a finger still means a window button (they are small; a slot's cell is
/// 18 pixels): the nearest one within it.
const WINDOW_REACH: f32 = 3.0;

/// An anchor as the share of the width and height it is at.
fn anchor_point(anchor: Anchor) -> (f32, f32) {
    match anchor {
        Anchor::TopLeft => (0.0, 0.0),
        Anchor::TopMiddle => (0.5, 0.0),
        Anchor::TopRight => (1.0, 0.0),
        Anchor::LeftMiddle => (0.0, 0.5),
        Anchor::Center => (0.5, 0.5),
        Anchor::RightMiddle => (1.0, 0.5),
        Anchor::BottomLeft => (0.0, 1.0),
        Anchor::BottomMiddle => (0.5, 1.0),
        Anchor::BottomRight => (1.0, 1.0),
    }
}

/// A box `w` by `h` placed in `area` (x, y, width, height) from `anchor`, offset by `x`, `y`.
fn anchored_in(anchor: Anchor, area: [f32; 4], x: f32, y: f32, w: f32, h: f32) -> [f32; 4] {
    let (fx, fy) = anchor_point(anchor);
    let (w, h) = (w.floor(), h.floor());
    [
        (area[0] + (area[2] - w) * fx + x).floor(),
        (area[1] + (area[3] - h) * fy + y).floor(),
        w,
        h,
    ]
}

/// A button's box on a screen `sw` by `sh` GUI pixels.
fn anchored(b: &ScreenButton, sw: f32, sh: f32) -> [f32; 4] {
    anchored_in(b.anchor, [0.0, 0.0, sw, sh], b.x, b.y, b.width, b.height)
}

fn inside(r: [f32; 4], (x, y): (f32, f32)) -> bool {
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}

impl Game<'_> {
    /// The atlas sprite a mod means by `sprite`: `hud/food_full` (vanilla's GUI sprites),
    /// `item/compass_16` (an item texture), `<namespace>:my_icon` (a content mod's or a pack's
    /// GUI sprite, `assets/<namespace>/textures/gui/sprites/my_icon.png`).
    pub(super) fn mod_sprite(&self, sprite: &str) -> Option<String> {
        let atlas = &self.gui.atlas;
        let (ns, path) = sprite.split_once(':').unwrap_or(("minecraft", sprite));
        [
            format!("gui/sprites/{path}"),
            format!("{ns}:gui/sprites/{path}"),
            sprite.to_owned(),
        ]
        .into_iter()
        .filter(|k| ns == "minecraft" || !k.starts_with("gui/"))
        .find(|k| atlas.sprites.contains_key(k))
    }

    /// The GUI's size now, in GUI pixels.
    fn widget_gui(&self) -> (f32, f32) {
        let batch = HudBatch::new(self.touch_size());
        (batch.width, batch.height)
    }

    /// The mods' buttons on the screen shown now (none under a mod's own screen): mod id, button
    /// and its box in GUI pixels.
    fn mod_screen_buttons(&self) -> Vec<(String, ScreenButton, [f32; 4])> {
        if !self.mods.mods.is_running() || self.mods.screen.is_some() {
            return Vec::new();
        }
        let Some(screen) = self.mod_screen_name() else {
            return Vec::new();
        };
        let (sw, sh) = self.widget_gui();
        self.mods
            .mods
            .widgets()
            .flat_map(|(id, w)| w.screen.iter().map(move |b| (id, b)))
            .filter(|(_, b)| b.screen == screen)
            .map(|(id, b)| (id.to_owned(), b.clone(), anchored(b, sw, sh)))
            .collect()
    }

    /// Over the game's screen: the mods' buttons, as its own (vanilla's, or the Horizon menus'),
    /// the hovered one's tooltip last.
    pub(super) fn draw_mod_screen_buttons(&mut self, batch: &mut HudBatch) {
        let buttons = self.mod_screen_buttons();
        if buttons.is_empty() {
            return;
        }
        let cursor = self.cursor;
        let touch = self.has_touch();
        let pressed = self.mods.pressed_button.clone();
        let mut tooltip = None;
        for (id, b, r) in &buttons {
            let hovered = !touch && inside(*r, cursor);
            let held = pressed.as_ref() == Some(&(id.clone(), b.id.clone()));
            let icon = (!b.icon.is_empty())
                .then(|| self.mod_sprite(&b.icon))
                .flatten();
            // The label after the icon, a square as tall as the button lets it be.
            let side = (r[3] - 4.0).clamp(0.0, 16.0);
            let text_left = if icon.is_some() { side + 4.0 } else { 0.0 };
            let atlas = &self.gui.atlas;
            let m = Measure(&atlas.font);
            let mut label = rich_text(&b.label);
            let room = r[2] as i32 - 4 - text_left as i32;
            if m.width(&label) > room {
                label = m.head(&label, room);
            }
            let mut draws = Vec::new();
            screen_draw::button(
                &mut draws,
                &m,
                &label,
                r[0] as i32,
                r[1] as i32,
                r[2] as i32,
                r[3] as i32,
                b.active,
                hovered || held,
            );
            // The label in the middle of what the icon leaves.
            for d in &mut draws {
                if let Draw::Text { x, .. } = d {
                    *x += (text_left / 2.0) as i32;
                }
            }
            screen_draw::paint(batch, atlas, &draws);
            if let Some(icon) = icon {
                let tint = if b.active {
                    u32::MAX
                } else {
                    rgba(160, 160, 160, 255)
                };
                let (x, y) = (r[0] + 3.0, r[1] + ((r[3] - side) / 2.0).floor());
                horizon_client::ui::sprite_sized(
                    batch,
                    &self.gui.atlas,
                    &icon,
                    x,
                    y,
                    side,
                    side,
                    tint,
                );
            }
            if hovered && !b.tooltip.is_empty() {
                tooltip = Some(b.tooltip.clone());
            }
        }
        if let Some(lines) = tooltip {
            let draws = [Draw::Tooltip {
                lines: lines.iter().map(|l| rich_text(l)).collect(),
                x: cursor.0 as i32,
                y: cursor.1 as i32,
            }];
            screen_draw::paint(batch, &self.gui.atlas, &draws);
        }
    }

    /// A left click on the game's screen: a mod's button under the cursor takes it (its press,
    /// then its release when the button comes up). True when one did.
    pub(super) fn mod_screen_button_click(&mut self, pressed: bool) -> bool {
        if !pressed {
            return match self.mods.pressed_button.take() {
                Some((id, button)) => {
                    self.mod_bind(&id, &button, false);
                    true
                }
                None => false,
            };
        }
        let cursor = self.cursor;
        let hit = self
            .mod_screen_buttons()
            .into_iter()
            .rev()
            .find(|(_, _, r)| inside(*r, cursor))
            .map(|(id, b, _)| (id, b.id, b.active))
            .or_else(|| {
                self.mod_window_button_at(cursor)
                    .map(|(id, b)| (id, b.id, true))
            });
        let Some((id, button, active)) = hit else {
            return false;
        };
        if active {
            self.audio
                .play("minecraft:ui.button.click", None, 0.25, 1.0);
            self.mod_bind(&id, &button, true);
            self.mods.pressed_button = Some((id, button));
        }
        true
    }

    /// The open menu's kind as `slots.get-menu` names it (`minecraft:generic_9x3`), and how many
    /// slots it has.
    fn open_menu_shape(&mut self) -> Option<(String, usize)> {
        let mut shape = None;
        // Read, nothing kept.
        self.menu_session(|m, _, _| {
            shape = Some((m.kind().registry_name(), super::remote::menu_items(m).len()));
            Some(false)
        });
        if self.screen == Screen::Inventory
            && let Some((_, name)) = self.remote.as_ref().and_then(|r| r.menu.as_ref())
        {
            return shape.map(|(_, n)| (name.clone(), n));
        }
        shape
    }

    /// The parts' boxes on a mod's screen, from the slots it says it draws where
    /// (`screens.set-slots`): the menu's own slots, and the player's inventory and hotbar while
    /// its 27 main slots show, by the ids vanilla's menus give them.
    fn declared_parts(&self, kind: &str, size: usize) -> (Option<[f32; 4]>, Option<[f32; 4]>) {
        let (own, main) = if kind == "minecraft:inventory" {
            (0..0, 9..36)
        } else {
            let own = size.saturating_sub(36);
            (0..own, own..own + 27)
        };
        let player = main.start..main.start + 36;
        let (mut container, mut inventory, mut shown) = (None, None, false);
        let grow = |part: &mut Option<[f32; 4]>, x: f32, y: f32| {
            let cell = [x - 1.0, y - 1.0, x + 17.0, y + 17.0];
            *part = Some(part.map_or(cell, |p| {
                [
                    p[0].min(cell[0]),
                    p[1].min(cell[1]),
                    p[2].max(cell[2]),
                    p[3].max(cell[3]),
                ]
            }));
        };
        for &(slot, x, y) in &self.mods.screen_slots {
            let slot = slot as usize;
            if own.contains(&slot) {
                grow(&mut container, x, y);
            } else if player.contains(&slot) && !(kind == "minecraft:inventory" && slot == 45) {
                shown |= main.contains(&slot);
                grow(&mut inventory, x, y);
            }
        }
        let rect = |p: [f32; 4]| [p[0], p[1], p[2] - p[0], p[3] - p[1]];
        (container.map(rect), inventory.filter(|_| shown).map(rect))
    }

    /// The boxes window buttons are placed from on the screen open (x, y, width, height in GUI
    /// pixels): the cells of the container's slots, and of the player's inventory and hotbar
    /// while its 27 main slots show.
    fn window_parts(&self) -> (Option<[f32; 4]>, Option<[f32; 4]>) {
        let batch = HudBatch::new(self.touch_size());
        let (ox, oy) = self.inventory_origin(&batch);
        let grow = |part: &mut Option<[f32; 4]>, (sx, sy): (f32, f32)| {
            let cell = [ox + sx - 1.0, oy + sy - 1.0, ox + sx + 17.0, oy + sy + 17.0];
            *part = Some(part.map_or(cell, |p| {
                [
                    p[0].min(cell[0]),
                    p[1].min(cell[1]),
                    p[2].max(cell[2]),
                    p[3].max(cell[3]),
                ]
            }));
        };
        let (mut container, mut inventory, mut main) = (None, None, false);
        for i in self.slots() {
            match i {
                0..36 => {
                    main |= i >= 9;
                    grow(&mut inventory, self.slot_position(i));
                }
                300..500 => grow(&mut container, self.slot_position(i)),
                _ => {}
            }
        }
        let rect = |p: [f32; 4]| [p[0], p[1], p[2] - p[0], p[3] - p[1]];
        (container.map(rect), inventory.filter(|_| main).map(rect))
    }

    /// The mods' window buttons on the screen open (a game screen with slots, or a mod's screen
    /// that says where it draws them): mod id, button and its box in GUI pixels.
    fn mod_window_buttons(&mut self) -> Vec<(String, WindowButton, [f32; 4])> {
        let over_mod = self.mods.screen.is_some();
        if !self.mods.mods.is_running()
            || self.title.is_some()
            || (over_mod && self.mods.screen_slots.is_empty())
            || (!over_mod && !matches!(self.screen, Screen::Inventory | Screen::Creative))
        {
            return Vec::new();
        }
        let buttons: Vec<(String, WindowButton)> = self
            .mods
            .mods
            .widgets()
            .flat_map(|(id, w)| w.window.iter().map(move |b| (id.to_owned(), b.clone())))
            .collect();
        if buttons.is_empty() {
            return Vec::new();
        }
        let (kind, size) = self.open_menu_shape().unwrap_or_default();
        let (container, inventory) = if over_mod {
            self.declared_parts(&kind, size)
        } else {
            self.window_parts()
        };
        buttons
            .into_iter()
            .filter(|(_, b)| b.menus.is_empty() || b.menus.contains(&kind))
            .filter_map(|(id, b)| {
                let area = match b.part {
                    WindowPart::Container => container?,
                    WindowPart::Inventory => inventory?,
                };
                let r = anchored_in(b.anchor, area, b.x, b.y, b.width, b.height);
                Some((id, b, r))
            })
            .collect()
    }

    /// The window button at `at`: the one under it, else on a touch screen the nearest whose
    /// box a little larger holds it.
    fn mod_window_button_at(&mut self, at: (f32, f32)) -> Option<(String, WindowButton)> {
        let buttons = self.mod_window_buttons();
        if let Some(i) = buttons.iter().rposition(|(_, _, r)| inside(*r, at)) {
            let (id, b, _) = buttons.into_iter().nth(i)?;
            return Some((id, b));
        }
        if !self.has_touch() {
            return None;
        }
        let distance = |r: &[f32; 4]| {
            let (dx, dy) = (at.0 - (r[0] + r[2] / 2.0), at.1 - (r[1] + r[3] / 2.0));
            dx * dx + dy * dy
        };
        buttons
            .into_iter()
            .filter(|(_, _, r)| {
                let reach = [
                    r[0] - WINDOW_REACH,
                    r[1] - WINDOW_REACH,
                    r[2] + 2.0 * WINDOW_REACH,
                    r[3] + 2.0 * WINDOW_REACH,
                ];
                inside(reach, at)
            })
            .min_by(|a, b| distance(&a.2).total_cmp(&distance(&b.2)))
            .map(|(id, b, _)| (id, b))
    }

    /// The mods' window buttons over the window: each its icon, the highlighted one under the
    /// mouse or while pressed (or a plain button with its label while the icon is not in the
    /// atlas), the hovered one's tooltip last. Over a game screen's window (`over_mod_screen`
    /// false), or over a mod's screen, drawn after it.
    pub(super) fn draw_mod_window_buttons(&mut self, batch: &mut HudBatch, over_mod_screen: bool) {
        if self.mods.screen.is_some() != over_mod_screen {
            return;
        }
        let buttons = self.mod_window_buttons();
        if buttons.is_empty() {
            return;
        }
        let cursor = self.cursor;
        let touch = self.has_touch();
        let pressed = self.mods.pressed_button.clone();
        let mut tooltip = None;
        for (id, b, r) in &buttons {
            let hovered = !touch && inside(*r, cursor);
            let lit = hovered || pressed.as_ref() == Some(&(id.clone(), b.id.clone()));
            let icon = (!b.icon.is_empty())
                .then(|| self.mod_sprite(&b.icon))
                .flatten();
            let atlas = &self.gui.atlas;
            if let Some(icon) = icon {
                let lit_icon = lit
                    .then(|| self.mod_sprite(&format!("{}_highlighted", b.icon)))
                    .flatten();
                let sprite = lit_icon.as_ref().unwrap_or(&icon);
                horizon_client::ui::sprite_sized(
                    batch,
                    atlas,
                    sprite,
                    r[0],
                    r[1],
                    r[2],
                    r[3],
                    u32::MAX,
                );
                if lit && lit_icon.is_none() {
                    batch.fill(atlas, r[0], r[1], r[2], r[3], rgba(255, 255, 255, 80));
                }
            } else {
                let m = Measure(&atlas.font);
                let mut draws = Vec::new();
                screen_draw::button(
                    &mut draws,
                    &m,
                    &rich_text(&b.label),
                    r[0] as i32,
                    r[1] as i32,
                    r[2] as i32,
                    r[3] as i32,
                    true,
                    lit,
                );
                screen_draw::paint(batch, atlas, &draws);
            }
            if hovered && !b.tooltip.is_empty() {
                tooltip = Some(b.tooltip.clone());
            }
        }
        if let Some(lines) = tooltip {
            let draws = [Draw::Tooltip {
                lines: lines.iter().map(|l| rich_text(l)).collect(),
                x: cursor.0 as i32,
                y: cursor.1 as i32,
            }];
            screen_draw::paint(batch, &self.gui.atlas, &draws);
        }
    }

    /// The mods' buttons on the touch controls: each keybind's (wide, its name on it, in a row
    /// at the left under the top buttons) unless the mod gave that id a look of its own, then the
    /// mod's own buttons. Each with the place it has until the player moves it.
    pub(super) fn mod_touch_buttons(&self) -> Vec<(ModFace, Place)> {
        if !self.mods.mods.is_running() {
            return Vec::new();
        }
        let [w, h] = self.touch_size();
        let (w, h) = (w as f32, h as f32);
        let scale = HudBatch::new([w as u32, h as u32]).scale;
        let unit = horizon_client::touch::unit(w, h);
        let atlas = &self.gui.atlas;
        let mut out = Vec::new();
        let place = |x: f32, y: f32, size: f32| Place {
            x,
            y,
            size,
            show: horizon_client::touch::Show::On,
            opacity: 1.0,
        };
        // Wide buttons are as tall as vanilla's 16 GUI pixels: the size is half that.
        let wide_size = WIDE_HEIGHT / 2.0 * scale / unit;
        let wide = |label: &str| {
            let width = HudBatch::text_width(atlas, label) + 10.0;
            Shape::Wide(width / WIDE_HEIGHT)
        };
        let widgets: Vec<_> = self.mods.mods.widgets().collect();
        let own = |id: &str, button: &str| {
            widgets
                .iter()
                .find(|(m, _)| *m == id)
                .and_then(|(_, w)| w.touch.iter().find(|b| b.id == button))
        };
        // The keybinds in a row, 4 GUI pixels apart, from 4 in at 20% of the height.
        let mut x = 4.0 * scale;
        for (id, bind) in self.mods.mods.keybinds() {
            if own(&id, &bind.id).is_some() {
                continue;
            }
            let label: String = bind.name.chars().take(14).collect();
            let shape = wide(&label);
            let half = shape.half(WIDE_HEIGHT / 2.0 * scale);
            let centre = [x + half[0], h * 0.2 + half[1]];
            x += 2.0 * half[0] + 3.0 * scale;
            out.push((
                ModFace {
                    key: format!("{id}:{}", bind.id),
                    label,
                    icon: None,
                    skin: None,
                    shape,
                },
                place(centre[0] / w, centre[1] / h, wide_size),
            ));
        }
        for (id, w) in &widgets {
            for b in &w.touch {
                let shape = match b.shape {
                    ModShape::Round => Shape::Round,
                    ModShape::Square => Shape::Square,
                    ModShape::Wide => wide(&b.label),
                };
                let icon = (!b.icon.is_empty())
                    .then(|| self.mod_sprite(&b.icon))
                    .flatten();
                let size = if b.shape == ModShape::Wide {
                    wide_size
                } else {
                    b.size
                };
                out.push((
                    ModFace {
                        key: format!("{id}:{}", b.id),
                        label: b.label.clone(),
                        icon,
                        skin: None,
                        shape,
                    },
                    place(b.x, b.y, size),
                ));
            }
        }
        out
    }

    /// The player's own buttons on the touch controls: wide, what they do on them, in a column
    /// until they are placed.
    pub(super) fn custom_touch_buttons(&self) -> Vec<(ModFace, Place)> {
        let Some(config) = &self.touch.config else {
            return Vec::new();
        };
        let [w, h] = self.touch_size();
        let (w, h) = (w as f32, h as f32);
        let scale = HudBatch::new([w as u32, h as u32]).scale;
        let size = WIDE_HEIGHT / 2.0 * scale / horizon_client::touch::unit(w, h);
        config
            .custom
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let label: String = b.label().chars().take(14).collect();
                let width = HudBatch::text_width(&self.gui.atlas, &label) + 10.0;
                let sprite = |name: &str| self.mod_sprite(&format!("starling:touch/{name}"));
                // Its own image goes dim while pressed; an icon has a pressed sprite.
                let skin = if b.image {
                    sprite(&format!("own_{}", b.id)).map(|own| (own.clone(), own))
                } else {
                    b.icon.as_deref().and_then(|icon| {
                        let up = sprite(&format!("custom_{icon}"))?;
                        Some((up, sprite(&format!("custom_{icon}_pressed"))?))
                    })
                };
                let face = ModFace {
                    key: format!("{}:{}", horizon_client::touch::CUSTOM_BUTTONS, b.id),
                    label,
                    icon: None,
                    shape: if skin.is_some() {
                        Shape::Square
                    } else {
                        Shape::Wide(width / WIDE_HEIGHT)
                    },
                    skin,
                };
                let place = Place {
                    x: 0.5,
                    y: 0.25 + 0.1 * (i % 5) as f32,
                    size: if face.skin.is_some() { 0.5 } else { size },
                    show: horizon_client::touch::Show::On,
                    opacity: 1.0,
                };
                (face, place)
            })
            .collect()
    }

    /// How one of the game's touch buttons looks now.
    pub(super) fn control_face(&self, button: Button) -> ControlFace {
        let name = button.id();
        let style = self
            .mods
            .mods
            .widgets()
            .flat_map(|(_, w)| w.styles.iter())
            .filter(|s| s.control == name)
            .last();
        let skin = self.mod_sprite(&format!("starling:touch/{name}")).map(|s| {
            let pressed = self.mod_sprite(&format!("starling:touch/{name}_pressed"));
            (s, pressed)
        });
        let Some(style) = style else {
            return ControlFace {
                label: button.label().to_owned(),
                icon: None,
                skin,
                shape: Shape::Round,
            };
        };
        ControlFace {
            label: if style.label.is_empty() {
                button.label().to_owned()
            } else {
                style.label.clone()
            },
            icon: (!style.icon.is_empty())
                .then(|| self.mod_sprite(&style.icon))
                .flatten(),
            skin: None,
            shape: match style.shape {
                ModShape::Square => Shape::Square,
                ModShape::Round | ModShape::Wide => Shape::Round,
            },
        }
    }
}
