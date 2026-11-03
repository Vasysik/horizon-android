use super::*;

#[test]
fn resolves_android_locales_and_explicit_overrides() {
    for locale in [
        "zh",
        "zh-CN",
        "zh_CN.UTF-8",
        "zh-SG",
        "zh-Hans",
        "zh-Hans-HK",
    ] {
        assert!(
            Preference::System.uses_chinese(&[locale.into()]),
            "{locale}"
        );
    }
    for locale in [
        "en-US",
        "fr-FR",
        "zh-TW",
        "zh-HK",
        "zh-MO",
        "zh-Hant-CN",
        "",
    ] {
        assert!(
            !Preference::System.uses_chinese(&[locale.into()]),
            "{locale}"
        );
    }
    assert!(!Preference::System.uses_chinese(&[]));
    assert!(!Preference::System.uses_chinese(&["en-US".into(), "zh-CN".into()]));
    assert!(Preference::Chinese.uses_chinese(&["en-US".into()]));
    assert!(!Preference::English.uses_chinese(&["zh-CN".into()]));
}

#[test]
fn preferences_survive_json_storage_and_future_values_fall_back_safely() {
    for (preference, stored) in [
        (Preference::System, "system"),
        (Preference::English, "en"),
        (Preference::Chinese, "zh_cn"),
    ] {
        let json = serde_json::to_string(&preference).unwrap();
        assert_eq!(json, format!("\"{stored}\""));
        assert_eq!(
            serde_json::from_str::<Preference>(&json).unwrap(),
            preference
        );
    }
    assert_eq!(
        serde_json::from_str::<Preference>("\"future-language\"").unwrap(),
        Preference::System
    );
}

fn placeholders(text: &str) -> std::collections::BTreeSet<&str> {
    text.split('{')
        .skip(1)
        .filter_map(|s| s.split_once('}').map(|(key, _)| key))
        .collect()
}

#[test]
fn catalog_keeps_every_placeholder() {
    for (source, translation) in chinese() {
        assert!(
            !translation.trim().is_empty(),
            "empty translation for {source}"
        );
        assert_eq!(placeholders(source), placeholders(translation), "{source}");
    }
}

#[test]
fn switching_updates_existing_messages_and_preserves_user_values() {
    let name = "Settings {count} 中文.zip";
    let message = Message::formatted("Removed {name}", &[("name", name)]);
    set(Preference::Chinese, &[]);
    assert_eq!(tr("Play"), "开始游戏");
    assert_eq!(message.render(), "已移除 Settings {count} 中文.zip");
    assert_eq!(
        tr("Unrecognized backend error"),
        "Unrecognized backend error"
    );
    assert_eq!(
        trf("{name} {count}", &[("name", "{count}"), ("count", "3")]),
        "{count} 3"
    );
    set(Preference::English, &[]);
    assert_eq!(tr("Play"), "Play");
    assert_eq!(message.render(), "Removed Settings {count} 中文.zip");
    set(Preference::System, &["zh-CN".into()]);
    assert_eq!(message.render(), "已移除 Settings {count} 中文.zip");
    set(Preference::System, &["en-US".into()]);
    assert_eq!(message.render(), "Removed Settings {count} 中文.zip");
    assert_eq!(interpolate("unfinished {name", &[]), "unfinished {name");
}
