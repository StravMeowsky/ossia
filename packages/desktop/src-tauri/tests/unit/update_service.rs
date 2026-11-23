use super::{Staged, UpdateService};
use ossia_types::{UpdateChannel, UpdateChannelPreference, UpdateSource, UpdateState};
use std::fs;
use std::sync::Arc;

#[test]
fn finish_on_quit_discards_an_unused_download() {
    let service = UpdateService::for_build("http://127.0.0.1:9", "micooz/ossia", "v0.1.0");
    let dir = tempfile::tempdir().unwrap();
    let staged = dir.path().join("ossia-update.zip");
    fs::write(&staged, b"downloaded update").unwrap();
    *service.staged.lock().unwrap() = Some(Staged {
        path: staged,
        version: "0.2.0".into(),
    });

    service.finish_on_quit();

    assert!(service.staged().is_none());
    assert!(!dir.path().exists());
}

#[tokio::test]
async fn update_checks_publish_status_and_configuration_changes_clear_it() {
    let home = tempfile::tempdir().unwrap();
    let mut service = UpdateService::for_build("http://127.0.0.1:9", "micooz/ossia", "v0.1.0");
    service.status_home = Some(Arc::new(home.path().to_owned()));
    service
        .check(UpdateSource::Github, None, UpdateChannel::Stable, false)
        .await;
    let status = ossia_backend::update_status::read(home.path())
        .unwrap()
        .unwrap();
    assert_eq!(status.state, UpdateState::Error);
    assert_eq!(status.source, UpdateSource::Github);
    service.set_channel(UpdateChannelPreference::Beta);
    assert!(ossia_backend::update_status::read(home.path())
        .unwrap()
        .is_none());

    let mut source = UpdateService::for_build("http://127.0.0.1:9", "micooz/ossia", "0.1.0-5-gabc");
    source.status_home = Some(Arc::new(home.path().to_owned()));
    source
        .check(UpdateSource::Ossia, None, UpdateChannel::Stable, false)
        .await;
    assert_eq!(
        ossia_backend::update_status::read(home.path())
            .unwrap()
            .unwrap()
            .state,
        UpdateState::Source
    );
    source.set_source(UpdateSource::Github);
    assert!(ossia_backend::update_status::read(home.path())
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn successful_check_publishes_available_version() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        let body = r#"[{"tag_name":"v0.2.0","draft":false,"prerelease":false,"body":"stable","assets":[]}]"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let home = tempfile::tempdir().unwrap();
    let mut service =
        UpdateService::for_build(&format!("http://{address}"), "micooz/ossia", "v0.1.0");
    service.status_home = Some(Arc::new(home.path().to_owned()));
    service
        .check(UpdateSource::Github, None, UpdateChannel::Stable, false)
        .await;
    server.join().unwrap();
    let status = ossia_backend::update_status::read(home.path())
        .unwrap()
        .unwrap();
    assert_eq!(status.state, UpdateState::Found);
    assert_eq!(status.latest.as_deref(), Some("0.2.0"));
    assert!(!status.stale);
}
