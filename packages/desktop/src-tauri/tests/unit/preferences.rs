use super::*;
use std::path::PathBuf;

#[test]
fn autostart_args_preserve_runtime_configuration() {
    let config = server::Config {
        home: PathBuf::from("/tmp/ossia home"),
        port: 3212,
        engine_root: PathBuf::from("/tmp/engine"),
        docs_root: PathBuf::from("/tmp/docs"),
        static_dir: Some(PathBuf::from("/tmp/ui")),
        dev_origin: Some("http://127.0.0.1:5173".into()),
        listen: Some("192.168.1.20".parse().unwrap()),
        public_url: Some("http://ossia.test:3212".into()),
    };
    assert_eq!(
        autostart_args(&config),
        [
            "--home",
            "/tmp/ossia home",
            "--port",
            "3212",
            "--engine-root",
            "/tmp/engine",
            "--docs-root",
            "/tmp/docs",
            "--static-dir",
            "/tmp/ui",
            "--dev-origin",
            "http://127.0.0.1:5173",
            "--listen",
            "192.168.1.20",
            "--public-url",
            "http://ossia.test:3212",
        ]
    );
}

#[test]
fn autostart_args_hide_verbatim_windows_paths() {
    let config = server::Config {
        home: PathBuf::from(r"\\?\E:\Projects\ossia\.ossia"),
        port: 3212,
        engine_root: PathBuf::from(r"\\?\E:\engine"),
        docs_root: PathBuf::from(r"\\?\E:\docs"),
        static_dir: Some(PathBuf::from(r"\\?\E:\ui")),
        dev_origin: None,
        listen: None,
        public_url: None,
    };
    let args = autostart_args(&config);

    assert_eq!(
        args,
        [
            "--home",
            r"E:\Projects\ossia\.ossia",
            "--port",
            "3212",
            "--engine-root",
            r"E:\engine",
            "--docs-root",
            r"E:\docs",
            "--static-dir",
            r"E:\ui",
        ]
    );
}

#[test]
fn startup_visibility_respects_preference_and_explicit_hide() {
    assert!(show_on_startup(StartupAction::Window, true));
    assert!(!show_on_startup(StartupAction::Tray, true));
    assert!(!show_on_startup(StartupAction::Window, false));
    assert!(!show_on_startup(StartupAction::Tray, false));
}

#[test]
fn language_parsing_uses_primary_subtag() {
    assert_eq!(
        UiLanguage::from_locale("zh-Hans-CN"),
        Some(UiLanguage::ZhCn)
    );
    assert_eq!(UiLanguage::from_locale("en_GB"), Some(UiLanguage::EnUs));
    assert_eq!(UiLanguage::from_locale("ja-JP"), None);
}

#[test]
fn tray_action_values_round_trip() {
    for action in [TrayClickAction::Menu, TrayClickAction::Window] {
        assert_eq!(action_from_value(action_value(action)), action);
    }
}
