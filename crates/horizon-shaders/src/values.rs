//! Option values a player picked (spec 8.1, 8.3): the value of a `#define NAME VALUE` or
//! `const TYPE NAME = VALUE;` line, a switch `#define NAME` on or commented out, and the sets of
//! them a `profile.NAME` of `shaders.properties` names. They take effect where the pack declares
//! them: [`Pack::set_values`](crate::Pack::set_values) rewrites those lines as the sources are
//! read, so the preprocessor, the constants and the switches all see the picked value.

use std::collections::BTreeMap;

use crate::Properties;

/// Values by option name: a number or word for a valued option, `true` or `false` for a switch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Values {
    values: BTreeMap<String, String>,
    /// The most a numeric option may say, whatever the pack or a profile asks.
    caps: BTreeMap<String, f64>,
}

impl Values {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.caps.is_empty()
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_owned(), value.to_owned());
    }

    /// Every declaration of `name` says at most `max`; a lower value and a value that isn't a
    /// plain number stay.
    pub fn cap(&mut self, name: &str, max: f64) {
        self.caps.insert(name.to_owned(), max);
    }

    /// The first of `wanted` the pack has as a profile, its name as the pack writes it. Names
    /// compare without case or `_`: packs write `VERYLOW`, `very_low` and `low`.
    pub fn first_profile(properties: &Properties, wanted: &[&str]) -> Option<String> {
        let plain = |name: &str| name.replace('_', "").to_ascii_lowercase();
        let names: Vec<&str> = properties
            .entries
            .iter()
            .filter_map(|e| e.key.strip_prefix("profile."))
            .collect();
        wanted.iter().find_map(|want| {
            names
                .iter()
                .find(|name| plain(name) == plain(want))
                .map(|name| (*name).to_owned())
        })
    }

    /// The pack's profiles, lightest first, each with its cost. A profile's cost is how far its
    /// values take the pack's options from their defaults toward more work: a switch turned on
    /// costs 1 and one turned off saves 1; a valued option costs its place between the lowest
    /// and highest value it may take, relative to the default's (a bigger shadow map, more
    /// samples). Nothing here knows a pack or a profile's name; a profile that sets none of
    /// the pack's options, or only words (see `is_scale`), has no cost to compare and is left out.
    pub fn profiles_by_cost(
        properties: &Properties,
        options: &BTreeMap<String, crate::PackOption>,
    ) -> Vec<(String, f64)> {
        let mut out = Vec::new();
        for entry in &properties.entries {
            let Some(name) = entry.key.strip_prefix("profile.") else {
                continue;
            };
            let Some(profile) = Self::profile(properties, name) else {
                continue;
            };
            let (mut cost, mut known) = (0.0, 0);
            for (option, value) in &profile.values {
                let Some(o) = options.get(option) else {
                    continue;
                };
                // Only a switch or a number says how much work a profile asks for; a word
                // (`TONEMAP=Hejl`) is a choice, and a profile made of choices alone (Bliss's
                // `profile.OLD_BLISS_TONEMAP`, a label in the profile list) ranks nothing.
                known += usize::from(is_scale(o));
                cost += option_cost(o, value);
            }
            if known > 0 {
                out.push((name.to_owned(), cost));
            }
        }
        out.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        out.dedup_by(|a, b| a.0 == b.0);
        out
    }

    /// `NAME=VALUE` (or OptiFine's `NAME:VALUE`), `NAME` (a switch on) and `!NAME` (off),
    /// separated by spaces or commas, as a profile lists them; later ones win.
    pub fn parse_list(&mut self, list: &str) {
        for word in list.split([' ', ',', '\t']).filter(|w| !w.is_empty()) {
            if let Some(name) = word.strip_prefix('!') {
                self.set(name, "false");
            } else if let Some((name, value)) = word.split_once(['=', ':']) {
                self.set(name, value);
            } else {
                self.set(word, "true");
            }
        }
    }

    /// The values `profile.NAME` sets (spec 8.3): the profiles it names first, then its own.
    /// None when the pack has no such profile.
    pub fn profile(properties: &Properties, name: &str) -> Option<Self> {
        let mut out = Self::default();
        out.add_profile(properties, name, 0).then_some(out)
    }

    fn add_profile(&mut self, properties: &Properties, name: &str, depth: u8) -> bool {
        let Some(list) = properties.get(&format!("profile.{name}")) else {
            return false;
        };
        let (inherited, own): (Vec<&str>, Vec<&str>) = list
            .split_whitespace()
            .partition(|w| w.starts_with("profile."));
        if depth < 8 {
            for other in inherited {
                self.add_profile(properties, &other["profile.".len()..], depth + 1);
            }
        }
        self.parse_list(&own.join(" "));
        true
    }

    /// `text` with every declaration of an option set here carrying its value.
    pub fn apply(&self, text: &str) -> String {
        if self.is_empty() {
            return text.to_owned();
        }
        let mut out = String::with_capacity(text.len());
        for line in text.split_inclusive('\n') {
            let (body, end) = match line.strip_suffix('\n') {
                Some(b) => b.strip_suffix('\r').map_or((b, "\n"), |b| (b, "\r\n")),
                None => (line, ""),
            };
            match self.rewrite(body) {
                Some(new) => out.push_str(&new),
                None => out.push_str(body),
            }
            out.push_str(end);
        }
        out
    }

    /// The line declaring an option set here, rewritten; None for any other line.
    fn rewrite(&self, line: &str) -> Option<String> {
        let indent = &line[..line.len() - line.trim_start().len()];
        let rest = line.trim_start();
        let (commented, rest) = match rest.strip_prefix("//") {
            Some(r) => (true, r.trim_start()),
            None => (false, rest),
        };
        if let Some(after) = rest.strip_prefix("#define") {
            let after = after.strip_prefix([' ', '\t'])?.trim_start();
            let name_len = identifier(after);
            let (name, tail) = after.split_at(name_len);
            // What follows the name: a value, then perhaps a comment.
            let (old, comment) = split_comment(tail);
            if old.trim().is_empty() {
                // A switch.
                let on = match self.values.get(name)?.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return None,
                };
                let prefix = if on { "" } else { "//" };
                return Some(format!("{indent}{prefix}#define {name}{tail}"));
            }
            if commented {
                return None;
            }
            let value = self.declared(name, old.trim())?;
            return Some(format!("{indent}#define {name} {value}{}", spaced(comment)));
        }
        if commented {
            return None;
        }
        let after = rest
            .strip_prefix("const")?
            .strip_prefix([' ', '\t'])?
            .trim_start();
        let type_len = identifier(after);
        let after_type = after[type_len..].trim_start();
        let name_len = identifier(after_type);
        let name = &after_type[..name_len];
        let after_name = after_type[name_len..].trim_start().strip_prefix('=')?;
        let (old, tail) = after_name.split_once(';')?;
        let value = self.declared(name, old.trim())?;
        Some(format!(
            "{indent}const {} {name} = {value};{tail}",
            &after[..type_len]
        ))
    }

    /// What the declaration of `name`, now saying `old`, should say: the picked value, kept under
    /// its cap. None leaves the line as it is.
    pub fn declared(&self, name: &str, old: &str) -> Option<String> {
        let picked = self.values.get(name);
        let now = picked.map_or(old, String::as_str);
        if let Some(&cap) = self.caps.get(name)
            && let Ok(value) = now.trim_end_matches(['f', 'F']).parse::<f64>()
            && value > cap
        {
            // In the form the line has: a `const int` takes no `1024.0`.
            return Some(if now.contains('.') {
                format!("{cap:?}")
            } else {
                format!("{cap}")
            });
        }
        picked.cloned()
    }
}

/// The length of the identifier `text` starts with.
fn identifier(text: &str) -> usize {
    text.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len())
}

/// A define's tail as its value and its `//` comment (with the `//`).
fn split_comment(tail: &str) -> (&str, &str) {
    match tail.find("//") {
        Some(i) => (&tail[..i], &tail[i..]),
        None => (tail, ""),
    }
}

fn spaced(comment: &str) -> String {
    if comment.is_empty() {
        String::new()
    } else {
        format!(" {comment}")
    }
}

/// Whether option `o` runs along a scale of work: a switch, values that are all numbers, or
/// words of quality (`LOW MEDIUM HIGH`) in their order. A list of names (tonemaps, palettes)
/// is a choice, not a scale.
fn is_scale(o: &crate::PackOption) -> bool {
    const LEVELS: [&str; 16] = [
        "off", "none", "potato", "minimal", "low", "lowest", "medium", "mid", "normal", "high",
        "highest", "ultra", "extreme", "max", "fast", "fancy",
    ];
    let number = |v: &str| v.trim().trim_end_matches(['f', 'F']).parse::<f64>().is_ok();
    let level = |v: &str| {
        let plain: String = v
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
            .to_ascii_lowercase();
        LEVELS.contains(&plain.as_str())
    };
    o.switch
        || (o.values.len() >= 2 && o.values.iter().all(|v| number(v)))
        || o.values.iter().filter(|v| level(v)).count() >= 2
}

/// What setting option `o` to `value` costs against its default, see `profiles_by_cost`.
fn option_cost(o: &crate::PackOption, value: &str) -> f64 {
    let on = |v: &str| f64::from(u8::from(v.trim() == "true"));
    if o.switch {
        return on(value) - on(&o.default);
    }
    let number = |v: &str| v.trim().trim_end_matches(['f', 'F']).parse::<f64>().ok();
    let numbers: Vec<f64> = o.values.iter().filter_map(|v| number(v)).collect();
    if numbers.len() >= 2 && numbers.len() == o.values.len() {
        let (lo, hi) = numbers
            .iter()
            .fold((f64::MAX, f64::MIN), |(l, h), &n| (l.min(n), h.max(n)));
        if let (Some(v), Some(d)) = (number(value), number(&o.default))
            && hi > lo
        {
            return (v - d) / (hi - lo);
        }
        return 0.0;
    }
    // Words (`LOW MEDIUM HIGH`): their place in the list.
    let place = |v: &str| o.values.iter().position(|x| x.trim() == v.trim());
    match (place(value), place(&o.default)) {
        (Some(v), Some(d)) if o.values.len() > 1 => {
            (v as f64 - d as f64) / (o.values.len() - 1) as f64
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Macros;

    #[test]
    fn a_picked_value_replaces_the_declared_one() {
        let mut values = Values::default();
        values.parse_list("SHADOW_QUALITY=0 shadowDistance=64.0 BLOOM !FXAA");
        let source = "    #define SHADOW_QUALITY 2 //[-1 0 1 2]\n\
                      \tconst float shadowDistance = 192.0; //[64.0 192.0]\r\n\
                      //#define BLOOM // bloom\n\
                      #define FXAA\n\
                      #define OTHER 3\n\
                      // #define SHADOW_QUALITY 5\n\
                      #if SHADOW_QUALITY == 2\n";
        assert_eq!(
            values.apply(source),
            "    #define SHADOW_QUALITY 0 //[-1 0 1 2]\n\
             \tconst float shadowDistance = 64.0; //[64.0 192.0]\r\n\
             #define BLOOM // bloom\n\
             //#define FXAA\n\
             #define OTHER 3\n\
             // #define SHADOW_QUALITY 5\n\
             #if SHADOW_QUALITY == 2\n"
        );
    }

    #[test]
    fn a_profile_takes_the_profiles_it_names_then_its_own() {
        let text = "profile.LOW = SHADOW_QUALITY=0 shadowDistance=96.0 !SSAO\n\
                    profile.POTATO = profile.LOW shadowDistance=64.0\n";
        let properties = Properties::parse(text, &Macros::standard());
        let potato = Values::profile(&properties, "POTATO").unwrap();
        let mut expected = Values::default();
        expected.parse_list("SHADOW_QUALITY=0 shadowDistance=64.0 !SSAO");
        assert_eq!(potato, expected);
        assert!(Values::profile(&properties, "ULTRA").is_none());
    }

    #[test]
    fn a_cap_lowers_what_is_above_it_and_nothing_else() {
        let mut values = Values::default();
        values.cap("shadowMapResolution", 1024.0);
        values.cap("shadowDistance", 96.0);
        values.parse_list("shadowDistance=192.0");
        let source = "const int shadowMapResolution = 4096;\n\
                      const int shadowMapResolution = 512;\n\
                      const int shadowMapResolution = RES * 2;\n\
                      //const int shadowMapResolution = 8192;\n\
                      const float shadowDistance = 64.0;\n\
                      #define shadowMapResolution 2048 //[1024 2048]\n\
                      #define OTHER 4096\n";
        assert_eq!(
            values.apply(source),
            "const int shadowMapResolution = 1024;\n\
             const int shadowMapResolution = 512;\n\
             const int shadowMapResolution = RES * 2;\n\
             //const int shadowMapResolution = 8192;\n\
             const float shadowDistance = 96.0;\n\
             #define shadowMapResolution 1024 //[1024 2048]\n\
             #define OTHER 4096\n"
        );
    }

    #[test]
    fn the_first_wanted_profile_the_pack_has_is_found_however_it_is_written() {
        let text = "profile.VERY_LOW = !SSAO\nprofile.low = SSAO\nprofile.ULTRA = SSAO\n";
        let properties = Properties::parse(text, &Macros::standard());
        let found = |wanted| Values::first_profile(&properties, wanted);
        assert_eq!(found(&["potato", "verylow", "low"]).unwrap(), "VERY_LOW");
        assert_eq!(found(&["LOW"]).unwrap(), "low");
        assert_eq!(found(&["potato"]), None);
    }

    #[test]
    fn profiles_are_ranked_by_what_they_ask_of_the_options() {
        let option = |name: &str, default: &str, values: &[&str], switch: bool| {
            (
                name.to_owned(),
                crate::PackOption {
                    name: name.to_owned(),
                    default: default.to_owned(),
                    values: values.iter().map(|v| (*v).to_owned()).collect(),
                    switch,
                    tested: true,
                },
            )
        };
        let options: BTreeMap<_, _> = [
            option("SSAO", "true", &["false", "true"], true),
            option(
                "shadowMapResolution",
                "2048",
                &["512", "1024", "2048", "4096"],
                false,
            ),
            option("QUALITY", "MEDIUM", &["LOW", "MEDIUM", "HIGH"], false),
            option("TONEMAP", "AgX", &["AgX", "Hejl", "ACES"], false),
        ]
        .into();
        let text = "profile.Fancy = shadowMapResolution=4096 QUALITY=HIGH\n\
                    profile.Fast = !SSAO shadowMapResolution=512\n\
                    profile.Mid = QUALITY=LOW\n\
                    profile.Label = TONEMAP=Hejl\n\
                    profile.Mystery = UNKNOWN_OPTION=3\n\
                    profile.Custom =\n";
        let properties = Properties::parse(text, &Macros::standard());
        let ranked = Values::profiles_by_cost(&properties, &options);
        let names: Vec<&str> = ranked.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(names, ["Fast", "Mid", "Fancy"]);
    }

    #[test]
    fn a_value_may_follow_a_colon() {
        let mut colon = Values::default();
        colon.parse_list("REFLECTION_MODE:1 BLOOM_QUALITY:0.25 !CLOUDS");
        let mut equals = Values::default();
        equals.parse_list("REFLECTION_MODE=1 BLOOM_QUALITY=0.25 !CLOUDS");
        assert_eq!(colon, equals);
    }
}
