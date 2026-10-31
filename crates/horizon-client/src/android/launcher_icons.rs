//! The design system's pixel icons (Pixelarticons, `horizon_theme::icons`) for the
//! Android launcher. Every icon is a 24-unit grid of axis-aligned squares, so the
//! launcher fills grid cells with plain quads instead of rasterizing SVG.
use horizon_client::ui_theme::Ui;
use std::{collections::HashMap, sync::OnceLock};

/// One bit per column (bit `x`), one word per row.
type Grid = [u32; 24];

fn grids() -> &'static HashMap<&'static str, Grid> {
    static GRIDS: OnceLock<HashMap<&'static str, Grid>> = OnceLock::new();
    GRIDS.get_or_init(|| {
        horizon_theme::icons::ALL
            .iter()
            .map(|(name, svg)| (*name, rasterize(&String::from_utf8_lossy(svg))))
            .collect()
    })
}

/// Outlines of every `d="..."` path in the SVG, one polygon per subpath.
fn outlines(svg: &str) -> Vec<Vec<[f32; 2]>> {
    let mut polygons = Vec::new();
    for chunk in svg.split(" d=\"").skip(1) {
        let data = chunk.split('"').next().unwrap_or_default();
        let mut tokens = Vec::new();
        let mut number = String::new();
        for ch in data.chars() {
            let starts_number = ch == '-' && !number.is_empty() && !number.ends_with('e');
            if ch.is_ascii_alphabetic() || ch == ',' || ch.is_whitespace() || starts_number {
                if !number.is_empty() {
                    tokens.push(Err(number.parse::<f32>().unwrap_or(0.0)));
                    number.clear();
                }
                if ch.is_ascii_alphabetic() {
                    tokens.push(Ok(ch));
                } else if starts_number {
                    number.push(ch);
                }
            } else {
                number.push(ch);
            }
        }
        if !number.is_empty() {
            tokens.push(Err(number.parse::<f32>().unwrap_or(0.0)));
        }
        let (mut pos, mut start) = ([0.0f32; 2], [0.0f32; 2]);
        let mut current: Vec<[f32; 2]> = Vec::new();
        let mut command = 'M';
        let mut i = 0;
        let mut first_pair = true;
        while i < tokens.len() {
            if let Ok(c) = tokens[i] {
                command = c;
                i += 1;
                first_pair = true;
                if matches!(c, 'Z' | 'z') {
                    if current.len() > 2 {
                        polygons.push(std::mem::take(&mut current));
                    }
                    current.clear();
                    pos = start;
                }
                continue;
            }
            let value = |k: usize| match tokens.get(k) {
                Some(Err(v)) => *v,
                _ => 0.0,
            };
            match command {
                'M' | 'm' | 'L' | 'l' => {
                    let mut next = [value(i), value(i + 1)];
                    if command.is_ascii_lowercase() {
                        next = [pos[0] + next[0], pos[1] + next[1]];
                    }
                    if matches!(command, 'M' | 'm') && first_pair {
                        if current.len() > 2 {
                            polygons.push(std::mem::take(&mut current));
                        }
                        current.clear();
                        start = next;
                    }
                    pos = next;
                    current.push(pos);
                    i += 2;
                }
                'H' => {
                    pos[0] = value(i);
                    current.push(pos);
                    i += 1;
                }
                'h' => {
                    pos[0] += value(i);
                    current.push(pos);
                    i += 1;
                }
                'V' => {
                    pos[1] = value(i);
                    current.push(pos);
                    i += 1;
                }
                'v' => {
                    pos[1] += value(i);
                    current.push(pos);
                    i += 1;
                }
                _ => i += 1,
            }
            first_pair = false;
        }
        if current.len() > 2 {
            polygons.push(current);
        }
    }
    polygons
}

/// Fills each grid cell whose centre is inside the outlines (non-zero winding).
fn rasterize(svg: &str) -> Grid {
    let polygons = outlines(svg);
    let mut grid = [0u32; 24];
    for (row, bits) in grid.iter_mut().enumerate() {
        let py = row as f32 + 0.5;
        for column in 0..24 {
            let px = column as f32 + 0.5;
            let mut winding = 0;
            for polygon in &polygons {
                for (k, a) in polygon.iter().enumerate() {
                    let b = polygon[(k + 1) % polygon.len()];
                    if (a[1] <= py) != (b[1] <= py) {
                        let x = a[0] + (py - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                        if x > px {
                            winding += if b[1] > a[1] { 1 } else { -1 };
                        }
                    }
                }
            }
            if winding != 0 {
                *bits |= 1 << column;
            }
        }
    }
    grid
}

/// Draws icon `name` in a `size`-dp square at (`x`, `y`).
pub(super) fn draw(ui: &mut Ui, name: &str, x: f32, y: f32, size: f32, color: u32) {
    let Some(grid) = grids().get(name) else {
        return;
    };
    let unit = size / 24.0;
    for (row, bits) in grid.iter().enumerate() {
        let mut column = 0;
        while column < 24 {
            if bits & (1 << column) == 0 {
                column += 1;
                continue;
            }
            let begin = column;
            while column < 24 && bits & (1 << column) != 0 {
                column += 1;
            }
            ui.rounded(
                x + begin as f32 * unit,
                y + row as f32 * unit,
                (column - begin) as f32 * unit,
                unit,
                0.0,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_pixels() {
        for (name, svg) in horizon_theme::icons::ALL {
            let grid = rasterize(&String::from_utf8_lossy(svg));
            assert!(grid.iter().any(|row| *row != 0), "{name} is empty");
        }
    }

    #[test]
    fn play_icon_matches_its_outline() {
        let grid = grids()["play"];
        // The stem: x 7..9 for y 3..21.
        assert_eq!(grid[3] & 0b1_1000_0000, 0b1_1000_0000);
        assert_eq!(grid[20] & 0b1_1000_0000, 0b1_1000_0000);
        assert_eq!(grid[2], 0);
        // The tip: x 15..17 at y 11..13.
        assert_ne!(grid[11] & (1 << 15), 0);
        assert_eq!(grid[11] & (1 << 17), 0);
    }
}
