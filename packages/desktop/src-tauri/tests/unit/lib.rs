use super::*;

#[test]
fn quit_request_is_single_flight_and_waits_for_approval() {
    let quit = QuitState::new();
    assert!(quit.request());
    assert!(!quit.request());
    assert!(quit.stopping());
    assert!(!quit.approved());
    quit.approve();
    assert!(quit.approved());
}

#[tokio::test]
async fn status_before_runtime_discovery_is_not_an_error() {
    let root = tempfile::tempdir().unwrap();
    let config = server::Config {
        home: root.path().into(),
        port: 0,
        engine_root: root.path().join("engine"),
        docs_root: root.path().join("docs"),
        static_dir: None,
        dev_origin: None,
        listen: None,
        public_url: None,
    };
    let status = snapshot(&config).await;
    assert!(status.runtime.is_none());
    assert!(status.error.is_none(), "{:?}", status.error);

    // Invalid discovery data must still surface as a real configuration error.
    std::fs::create_dir(root.path().join("run")).unwrap();
    std::fs::write(root.path().join("run/runtime.json"), b"invalid").unwrap();
    let status = snapshot(&config).await;
    assert_eq!(status.error.unwrap().code, ErrorCode::StorageError);
}

#[tokio::test]
async fn snapshot_reports_normalized_data_paths() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("library");
    let engine = root.path().join("engine");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&engine).unwrap();
    let home = home.canonicalize().unwrap();
    let engine = engine.canonicalize().unwrap();
    let config = server::Config {
        home: home.clone(),
        port: 0,
        engine_root: engine.clone(),
        docs_root: root.path().join("docs"),
        static_dir: None,
        dev_origin: None,
        listen: None,
        public_url: None,
    };

    let status = snapshot(&config).await;

    assert_eq!(status.home, path_text(&home));
    assert_eq!(status.engine_root, path_text(&engine));
    assert!(!status.home.contains(r"\\?\"));
    assert!(!status.engine_root.contains(r"\\?\"));
}
