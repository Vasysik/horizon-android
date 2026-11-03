//! Mining, the other way to pass the time while the build service compiles (native_builds.rs), seen
//! from above: rock over the screen with a way in at the middle. A tap on a block beside the way in
//! digs at it: dirt gives at once, stone takes two, ore three. Ore shows once it is beside the way
//! in, and what was found stays counted. When there is nothing left, a new level comes.
use super::setup::fishing::Sound;
use horizon_client::ui_theme::{Face, Ui, color};
use std::time::{Duration, Instant};

/// A block, in dp.
const CELL: f32 = 44.0;
/// Left clear at the top of the screen, for the build's status.
const TOP: f32 = 72.0;
/// How long a find stays written.
const SAID: Duration = Duration::from_millis(1400);

const FLOOR: u32 = color::hex(0x1a1c22, 255);
const FLOOR_LIGHT: u32 = color::hex(0x23262e, 255);
const DIRT: [u32; 2] = [color::hex(0x6b4a2f, 255), color::hex(0x7a5636, 255)];
const STONE: [u32; 2] = [color::hex(0x5c5f66, 255), color::hex(0x6c6f77, 255)];
const CRACK: u32 = color::hex(0x24262b, 255);

#[derive(Clone, Copy, PartialEq)]
enum Block {
    Dug,
    Dirt,
    Stone,
    Ore(usize),
}

/// The ores, how often in a thousand blocks of rock, and their colour.
const ORES: [(&str, u32, u32); 5] = [
    ("Coal", 70, color::hex(0x17181c, 255)),
    ("Iron", 45, color::hex(0xd8af93, 255)),
    ("Gold", 20, color::hex(0xfcee4b, 255)),
    ("Redstone", 30, color::hex(0xe02020, 255)),
    ("Diamond", 8, color::hex(0x5decf5, 255)),
];

struct Cell {
    block: Block,
    hits: u8,
    /// Which shade, and where its specks go.
    speck: u32,
}

pub(super) struct BuildMining {
    cells: Vec<Cell>,
    columns: usize,
    rows: usize,
    seed: u32,
    found: [u32; ORES.len()],
    said: Option<(String, Instant)>,
    sounds: Vec<Sound>,
}

impl Default for BuildMining {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            columns: 0,
            rows: 0,
            seed: 0x51f1_5e11,
            found: [0; ORES.len()],
            said: None,
            sounds: Vec::new(),
        }
    }
}

impl BuildMining {
    fn random(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    /// A new level for a screen of `w` x `h` dp, dug out at the middle.
    fn level(&mut self, w: f32, h: f32) {
        self.columns = ((w / CELL) as usize).max(1);
        self.rows = (((h - TOP) / CELL) as usize).max(1);
        self.cells.clear();
        for _ in 0..self.columns * self.rows {
            let roll = self.random() % 1000;
            let speck = self.random();
            let mut at = 0;
            let mut block = if roll.is_multiple_of(7) {
                Block::Dirt
            } else {
                Block::Stone
            };
            for (i, ore) in ORES.iter().enumerate() {
                at += ore.1;
                if roll < at {
                    block = Block::Ore(i);
                    break;
                }
            }
            let hits = match block {
                Block::Dug | Block::Dirt => 1,
                Block::Stone => 2,
                Block::Ore(_) => 3,
            };
            self.cells.push(Cell { block, hits, speck });
        }
        let middle = self.rows / 2 * self.columns + self.columns / 2;
        self.cells[middle].block = Block::Dug;
    }

    /// The level fits the screen `w` x `h`, and there is rock left to reach.
    fn fit(&mut self, w: f32, h: f32) {
        let columns = ((w / CELL) as usize).max(1);
        let rows = (((h - TOP) / CELL) as usize).max(1);
        if columns != self.columns
            || rows != self.rows
            || self.cells.iter().all(|c| c.block == Block::Dug)
        {
            self.level(w, h);
        }
    }

    /// Whether the block at (`x`, `y`) touches the way in.
    fn reachable(&self, x: usize, y: usize) -> bool {
        let dug = |x: usize, y: usize| self.cells[y * self.columns + x].block == Block::Dug;
        (x > 0 && dug(x - 1, y))
            || (x + 1 < self.columns && dug(x + 1, y))
            || (y > 0 && dug(x, y - 1))
            || (y + 1 < self.rows && dug(x, y + 1))
    }

    /// A tap at `at` (dp) on a screen of `w` x `h`.
    pub(super) fn press(&mut self, at: [f32; 2], w: f32, h: f32) {
        self.fit(w, h);
        if at[1] < TOP {
            return;
        }
        let (x, y) = ((at[0] / CELL) as usize, ((at[1] - TOP) / CELL) as usize);
        if x >= self.columns || y >= self.rows || !self.reachable(x, y) {
            return;
        }
        let cell = &mut self.cells[y * self.columns + x];
        if cell.block == Block::Dug {
            return;
        }
        cell.hits = cell.hits.saturating_sub(1);
        if cell.hits > 0 {
            self.sounds.push(Sound::Blip);
            return;
        }
        let block = std::mem::replace(&mut cell.block, Block::Dug);
        if let Block::Ore(i) = block {
            self.found[i] += 1;
            self.said = Some((format!("{} found!", ORES[i].0), Instant::now()));
            self.sounds.push(Sound::Catch);
        } else {
            self.sounds.push(Sound::Plop);
        }
    }

    pub(super) fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// The rock over the screen, drawn from the level as last fitted (`prepare`).
    pub(super) fn draw(&self, ui: &mut Ui<'_>, w: f32, h: f32, now: Instant) {
        ui.fill(0.0, 0.0, w, h, FLOOR);
        for y in 0..self.rows {
            for x in 0..self.columns {
                let cell = &self.cells[y * self.columns + x];
                let (cx, cy) = (x as f32 * CELL, TOP + y as f32 * CELL);
                self.draw_cell(ui, cell, x, y, cx, cy);
            }
        }
        // What was found, along the bottom, and the last find over it.
        let mut text = String::new();
        for (ore, count) in ORES.iter().zip(self.found) {
            if count > 0 {
                text.push_str(&format!("{} {count}   ", ore.0));
            }
        }
        if text.is_empty() {
            text = "Tap the blocks beside the way in to dig".to_owned();
        }
        let width = ui.measure(Face::Regular, 13.0, &text) + 24.0;
        ui.rounded((w - width) / 2.0, h - 36.0, width, 26.0, 8.0, color::BG);
        ui.text_centered(Face::Regular, 13.0, w / 2.0, h - 32.0, color::TEXT, &text);
        if let Some((said, at)) = &self.said
            && now.duration_since(*at) < SAID
        {
            let width = ui.measure(Face::Semibold, 15.0, said) + 28.0;
            ui.rounded((w - width) / 2.0, h - 72.0, width, 30.0, 8.0, color::BG);
            ui.text_centered(Face::Semibold, 15.0, w / 2.0, h - 66.0, color::ACCENT, said);
        }
    }

    fn draw_cell(&self, ui: &mut Ui<'_>, cell: &Cell, x: usize, y: usize, cx: f32, cy: f32) {
        let px = CELL / 8.0;
        if cell.block == Block::Dug {
            if (x + y).is_multiple_of(2) {
                ui.fill(cx, cy, CELL, CELL, FLOOR_LIGHT);
            }
            return;
        }
        let near = self.reachable(x, y);
        let shade = (cell.speck & 1) as usize;
        let (base, mark) = match cell.block {
            Block::Dirt => (DIRT[shade], DIRT[1 - shade]),
            _ => (STONE[shade], STONE[1 - shade]),
        };
        ui.fill(cx, cy, CELL, CELL, base);
        // The specks of its texture: an ore's colour once it can be seen.
        let specks = match cell.block {
            Block::Ore(i) if near => ORES[i].2,
            _ => mark,
        };
        let mut bits = cell.speck;
        for _ in 0..6 {
            let (sx, sy) = ((bits & 7) as f32, ((bits >> 3) & 7) as f32);
            ui.fill(cx + sx * px, cy + sy * px, px * 1.5, px * 1.5, specks);
            bits = bits.rotate_right(6);
        }
        // Dug at: cracks.
        let full = match cell.block {
            Block::Stone => 2,
            Block::Ore(_) => 3,
            _ => 1,
        };
        if cell.hits < full {
            ui.fill(cx + 2.0 * px, cy + 3.0 * px, 4.0 * px, px * 0.6, CRACK);
            ui.fill(cx + 4.0 * px, cy + px, px * 0.6, 5.0 * px, CRACK);
        }
        // What can't be reached yet stays in the dark.
        if !near {
            ui.fill(cx, cy, CELL, CELL, color::with_alpha(FLOOR, 0.55));
        }
    }

    /// Before drawing, with the frame's screen.
    pub(super) fn prepare(&mut self, w: f32, h: f32) {
        self.fit(w, h);
    }
}
