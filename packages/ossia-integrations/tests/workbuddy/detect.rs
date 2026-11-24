use super::*;

#[test]
fn data_folder_must_be_one_component() {
    assert!(valid_data_folder(".workbuddy"));
    assert!(valid_data_folder(".workbuddy-ai"));
    assert!(!valid_data_folder("../escape"));
    assert!(!valid_data_folder("a/b"));
    assert!(!valid_data_folder(""));
}

#[test]
fn version_key_orders_numeric_segments() {
    assert!(version_key("22.22.2") > version_key("22.9.9"));
    assert!(version_key("22.22.2-6") > version_key("22.22.1"));
}

#[test]
fn node_executable_uses_the_newest_staged_runtime() {
    let root = tempfile::tempdir().unwrap();
    let older = root.path().join("binaries/node/versions/22.9.0/bin");
    let newer = root.path().join("binaries/node/versions/22.10.0/bin");
    fs::create_dir_all(&older).unwrap();
    fs::create_dir_all(&newer).unwrap();
    fs::write(older.join("node"), b"older").unwrap();
    fs::write(newer.join("node"), b"newer").unwrap();
    assert_eq!(node_executable(root.path()).unwrap(), newer.join("node"));
}

#[cfg(windows)]
#[test]
fn node_executable_accepts_the_windows_flat_layout() {
    let root = tempfile::tempdir().unwrap();
    let version = root.path().join("binaries/node/versions/22.22.2-6");
    fs::create_dir_all(&version).unwrap();
    fs::write(version.join("node.exe"), b"node").unwrap();
    assert_eq!(
        node_executable(root.path()).unwrap(),
        version.join("node.exe")
    );
}
