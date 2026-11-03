//! Android launcher localization, independent of the selected in-game language.
//! Like the desktop launcher, catalogs map English source text to translations.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    #[serde(rename = "en", alias = "english", alias = "en_us")]
    English,
    #[serde(rename = "zh_cn", alias = "zh-CN")]
    Chinese,
    #[default]
    #[serde(other)]
    System,
}

impl Preference {
    pub const ALL: [Self; 3] = [Self::System, Self::English, Self::Chinese];

    /// Match Simplified Chinese, preserving an explicit script over the region.
    /// Other Chinese variants fall back to English until they have a catalog.
    pub fn uses_chinese(self, locales: &[String]) -> bool {
        match self {
            Self::English => false,
            Self::Chinese => true,
            Self::System => locales.first().is_some_and(|locale| {
                let normalized = locale.to_ascii_lowercase().replace('_', "-");
                let parts: Vec<_> = normalized.split(['-', '.', '@']).collect();
                parts.first() == Some(&"zh")
                    && !parts.contains(&"hant")
                    && (parts.contains(&"hans")
                        || !parts.iter().any(|p| matches!(*p, "tw" | "hk" | "mo")))
            }),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => tr("Follow system"),
            Self::English => "English",
            Self::Chinese => "简体中文",
        }
    }
}

static CHINESE: AtomicBool = AtomicBool::new(false);

pub fn set(preference: Preference, locales: &[String]) {
    CHINESE.store(preference.uses_chinese(locales), Ordering::Relaxed);
}

fn chinese() -> &'static HashMap<String, String> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/android/lang/zh_cn.json"))
            .expect("validated Android launcher catalog")
    })
}

pub fn tr(source: &str) -> &str {
    if CHINESE.load(Ordering::Relaxed) {
        chinese().get(source).map(String::as_str).unwrap_or(source)
    } else {
        source
    }
}

/// Replace named placeholders in one pass: user-supplied values stay literal.
pub fn trf(source: &str, args: &[(&str, &str)]) -> String {
    interpolate(tr(source), args)
}

fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        result.push_str(&rest[..open]);
        rest = &rest[open..];
        let Some(close) = rest.find('}') else { break };
        let key = &rest[1..close];
        if let Some((_, value)) = args.iter().find(|(name, _)| *name == key) {
            result.push_str(value);
        } else {
            result.push_str(&rest[..=close]);
        }
        rest = &rest[close + 1..];
    }
    result.push_str(rest);
    result
}

/// Keep status text untranslated until drawing, so existing messages follow a
/// language switch too. Unknown backend errors are displayed verbatim.
#[derive(Default)]
pub struct Message {
    source: String,
    args: Vec<(String, String)>,
}
impl Message {
    pub fn formatted(source: &str, args: &[(&str, &str)]) -> Self {
        Self {
            source: source.into(),
            args: args
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        }
    }
    pub fn render(&self) -> String {
        let args: Vec<_> = self
            .args
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        trf(&self.source, &args)
    }
    pub fn clear(&mut self) {
        self.source.clear();
        self.args.clear();
    }
    pub fn contains(&self, text: &str) -> bool {
        self.source.contains(text)
    }
}
impl From<&str> for Message {
    fn from(source: &str) -> Self {
        Self::formatted(source, &[])
    }
}
impl From<String> for Message {
    fn from(source: String) -> Self {
        Self {
            source,
            args: vec![],
        }
    }
}
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

#[cfg(test)]
#[path = "i18n_tests.rs"]
mod tests;
