//! Font selection and metrics shared by UI drawing, measuring and truncation.
use ab_glyph::{Font as _, FontRef, GlyphId, PxScale, ScaleFont as _};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Regular,
    Medium,
    SemiBold,
    Mono,
}

pub(super) struct Fonts {
    faces: [FontRef<'static>; 5],
}
pub(super) struct Glyph {
    pub font: usize,
    pub id: GlyphId,
    pub advance: f32,
    pub kern: f32,
}
impl Fonts {
    pub fn load() -> Self {
        let f = |b: &'static [u8]| FontRef::try_from_slice(b).expect("bundled font");
        Self {
            faces: [
                f(horizon_theme::fonts::GEIST_REGULAR),
                f(horizon_theme::fonts::GEIST_MEDIUM),
                f(horizon_theme::fonts::GEIST_SEMIBOLD),
                f(horizon_theme::fonts::GEIST_MONO),
                f(horizon_theme::fonts::NOTO_SANS_SC),
            ],
        }
    }
    pub fn face(&self, face: Face) -> &FontRef<'static> {
        self.font(face as usize)
    }
    pub fn font(&self, index: usize) -> &FontRef<'static> {
        &self.faces[index]
    }
    pub fn scale(&self, face: Face, px: f32) -> PxScale {
        self.scale_at(face as usize, px)
    }
    pub fn scale_at(&self, index: usize, px: f32) -> PxScale {
        let font = self.font(index);
        PxScale::from(px * font.height_unscaled() / font.units_per_em().unwrap_or(1000.0))
    }
    fn select(&self, face: Face, ch: char) -> (usize, GlyphId) {
        let id = self.face(face).glyph_id(ch);
        if id.0 == 0 {
            let fallback = self.font(4).glyph_id(ch);
            if fallback.0 != 0 {
                return (4, fallback);
            }
        }
        (face as usize, id)
    }
    pub fn glyph(&self, face: Face, previous: Option<char>, ch: char, px: f32) -> Glyph {
        let (font, id) = self.select(face, ch);
        let scaled = self.font(font).as_scaled(self.scale_at(font, px));
        let kern = previous.map_or(0.0, |p| {
            let (previous_font, previous_id) = self.select(face, p);
            if previous_font == font {
                scaled.kern(previous_id, id)
            } else {
                0.0
            }
        });
        Glyph {
            font,
            id,
            advance: scaled.h_advance(id),
            kern,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offline_font_covers_every_localized_character() {
        let fonts = Fonts::load();
        let catalog: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../assets/android/lang/zh_cn.json")).unwrap();
        for text in catalog.values().map(String::as_str).chain([
            "简体中文",
            "存档：你好，世界！",
            "玩家张三",
        ]) {
            for face in [Face::Regular, Face::Medium, Face::SemiBold, Face::Mono] {
                for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
                    let glyph = fonts.glyph(face, None, ch, 16.0);
                    assert_ne!(glyph.id.0, 0, "missing {ch} in {text}");
                    assert!(glyph.advance > 0.0);
                    assert!(
                        fonts
                            .font(glyph.font)
                            .outline_glyph(glyph.id.with_scale(fonts.scale_at(glyph.font, 16.0)))
                            .is_some(),
                        "{ch} cannot be rasterized"
                    );
                }
            }
        }
    }

    #[test]
    fn fallback_metrics_use_the_selected_font_and_keep_latin_kerning() {
        let fonts = Fonts::load();
        let glyph = fonts.glyph(Face::Medium, Some('A'), '中', 20.0);
        assert_eq!(glyph.font, 4);
        assert_eq!(glyph.kern, 0.0);
        assert!((glyph.advance - 20.0).abs() < 0.01);
        let latin = fonts.glyph(Face::Medium, Some('A'), 'V', 20.0);
        let original = fonts
            .face(Face::Medium)
            .as_scaled(fonts.scale(Face::Medium, 20.0));
        assert_eq!(latin.font, Face::Medium as usize);
        assert_eq!(
            latin.kern,
            original.kern(original.glyph_id('A'), original.glyph_id('V'))
        );
    }
}
