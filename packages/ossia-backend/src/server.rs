use crate::*;
use serde_json::json;
use std::{
    collections::BTreeSet,
    ffi::OsString,
    io::ErrorKind,
    net::{Ipv4Addr, Ipv6Addr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Config {
    pub home: PathBuf,
    pub port: u16,
    pub engine_root: PathBuf,
    pub static_dir: Option<PathBuf>,
    pub docs_root: PathBuf,
    pub dev_origin: Option<String>,
    pub listen: Option<Ipv4Addr>,
    pub public_url: Option<String>,
}

#[derive(Debug)]
struct WebAccess {
    base_url: String,
    allowed_hosts: Vec<String>,
}

fn configured_listen(explicit: Option<Ipv4Addr>, lan_access: bool) -> Ipv4Addr {
    explicit.unwrap_or(if lan_access {
        Ipv4Addr::UNSPECIFIED
    } else {
        Ipv4Addr::LOCALHOST
    })
}

fn advertisable_lan_ipv4(address: Ipv4Addr) -> bool {
    !address.is_loopback()
        && !address.is_link_local()
        && !address.is_unspecified()
        && !address.is_broadcast()
        && !address.is_multicast()
}

fn local_ipv4_addresses() -> Result<Vec<Ipv4Addr>> {
    let mut addresses = if_addrs::get_if_addrs()
        .map_err(storage)?
        .into_iter()
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(address) if advertisable_lan_ipv4(address.ip) => {
                Some((interface.name, address.ip))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    addresses
        .sort_by_key(|(name, address)| (network_interface_rank(name, *address), address.octets()));
    let mut seen = BTreeSet::new();
    addresses.retain(|(_, address)| seen.insert(*address));
    Ok(addresses.into_iter().map(|(_, address)| address).collect())
}

fn network_interface_rank(name: &str, address: Ipv4Addr) -> u8 {
    let name = name.to_ascii_lowercase();
    if ["utun", "tun", "tap", "tailscale", "wg", "vpn"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        3
    } else if name.starts_with("en")
        || name.starts_with("eth")
        || name.starts_with("wlan")
        || name.contains("wi-fi")
        || name.contains("ethernet")
    {
        0
    } else if address.is_private() {
        1
    } else {
        2
    }
}

fn normalized_host(host: &str) -> String {
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase()
}

fn loopback_host(host: &str) -> bool {
    host == "localhost"
        || host == "::1"
        || host
            .parse::<Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
}

fn public_origin(url: &str, port: u16) -> Result<(String, String)> {
    let uri: axum::http::Uri = url.parse().map_err(|_| invalid("invalid public URL"))?;
    if uri.scheme_str() != Some("http")
        || uri.port_u16() != Some(port)
        || uri.path() != "/"
        || uri.query().is_some()
        || url.ends_with('/')
    {
        return Err(invalid(
            "public URL must be an explicit HTTP origin with the backend port and no path",
        ));
    }
    let host = uri
        .host()
        .ok_or_else(|| invalid("public URL has no host"))?;
    Ok((url.to_string(), normalized_host(host)))
}

fn resolve_web_access(
    explicit_listen: Option<Ipv4Addr>,
    lan_access: bool,
    public_url: Option<&str>,
    local: &[Ipv4Addr],
    port: u16,
) -> Result<WebAccess> {
    let listen = configured_listen(explicit_listen, lan_access);
    if listen.is_loopback() && public_url.is_some() {
        return Err(invalid("public URL requires a non-loopback listen address"));
    }
    let mut hosts = BTreeSet::from([
        "127.0.0.1".to_string(),
        "::1".to_string(),
        "localhost".to_string(),
    ]);
    let (base_url, public_host) = if let Some(url) = public_url {
        let (url, host) = public_origin(url, port)?;
        if loopback_host(&host) {
            return Err(invalid("public URL must not advertise a loopback host"));
        }
        (url, Some(host))
    } else if listen.is_loopback() {
        (format!("http://localhost:{port}"), None)
    } else if listen.is_unspecified() {
        let base_url = local
            .iter()
            .copied()
            .find(|address| advertisable_lan_ipv4(*address))
            .map_or_else(
                || format!("http://localhost:{port}"),
                |address| format!("http://{address}:{port}"),
            );
        (base_url, None)
    } else {
        (format!("http://{listen}:{port}"), None)
    };
    if let Some(host) = &public_host {
        hosts.insert(host.clone());
    }
    if !listen.is_loopback() {
        for address in local
            .iter()
            .filter(|address| advertisable_lan_ipv4(**address))
        {
            hosts.insert(address.to_string());
        }
        if !listen.is_unspecified() {
            hosts.insert(listen.to_string());
        }
    }
    Ok(WebAccess {
        base_url,
        allowed_hosts: hosts.into_iter().collect(),
    })
}

pub fn resolve_home(explicit: Option<PathBuf>) -> Result<PathBuf> {
    let path = explicit
        .or_else(|| std::env::var_os("OSSIA_HOME").map(PathBuf::from))
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".ossia"));
    if !path.is_absolute() {
        return Err(invalid("home must be an absolute path"));
    }
    canonical_home(&path)
}
pub fn canonical_home(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path.canonicalize().map_err(storage);
    }
    let parent = path.parent().ok_or_else(|| invalid("invalid home"))?;
    Ok(canonical_home(parent)?.join(path.file_name().ok_or_else(|| invalid("invalid home"))?))
}
fn secure_run(home: &Path) -> Result<PathBuf> {
    let path = home.join("run");
    if path.exists() {
        let meta = std::fs::symlink_metadata(&path).map_err(storage)?;
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(storage("run must be a real directory"));
        }
    }
    std::fs::create_dir_all(&path).map_err(storage)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).map_err(storage)?;
    }
    Ok(path)
}

pub async fn serve(config: Config) -> Result<()> {
    if let Some(origin) = &config.dev_origin {
        let uri: axum::http::Uri = origin
            .parse()
            .map_err(|_| invalid("invalid development Origin"))?;
        if uri.scheme_str() != Some("http")
            || !matches!(uri.host(), Some("127.0.0.1" | "localhost"))
            || uri.port_u16().is_none()
            || uri.path() != "/"
            || uri.query().is_some()
            || origin.ends_with('/')
        {
            return Err(invalid("development Origin must be an explicit loopback HTTP origin without trailing slash"));
        }
    }
    let library = ossia_library::Library::open(&config.home)?;
    crate::guide::outcome(library.home())?;
    ossia_integrations::launcher::publish(
        library.home(),
        &std::env::current_exe().map_err(storage)?,
    )?;
    let ui_preferences = crate::preferences::load(library.home())?;
    if ui_preferences.storage_auto_cleanup {
        if let Err(error) = library.storage_cleanup(ui_preferences.storage_cache_limit) {
            crate::startup_note(format!("ossia: startup storage cleanup failed: {error}"));
        }
    }
    let run = secure_run(library.home())?;
    let root = config.engine_root.clone();
    let scratch = run.join("scratch");
    let engine = tokio::task::spawn_blocking(move || {
        ossia_engine::Engine::verify(&root, &scratch, &ossia_engine::RunContext::default())
    })
    .await
    .map_err(storage)??;
    let listen = configured_listen(config.listen, ui_preferences.lan_access);
    let local = if listen.is_unspecified() {
        local_ipv4_addresses()?
    } else {
        Vec::new()
    };
    let listener = tokio::net::TcpListener::bind((listen, config.port))
        .await
        .map_err(|e| {
            AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("cannot bind {listen}:{}: {e}", config.port),
            )
        })?;
    let port = listener.local_addr().map_err(storage)?.port();
    let access = resolve_web_access(
        config.listen,
        ui_preferences.lan_access,
        config.public_url.as_deref(),
        &local,
        port,
    )?;
    let listener6 = match tokio::net::TcpListener::bind((Ipv6Addr::LOCALHOST, port)).await {
        Ok(listener) => Some(listener),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::AddrNotAvailable | ErrorKind::Unsupported
            ) =>
        {
            None
        }
        Err(error) => {
            return Err(AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("cannot bind requested loopback IPv6 port {port}: {error}"),
            ))
        }
    };
    let url = access.base_url.clone();
    let engine = crate::RuntimeEngine::new(
        engine,
        library.home().to_path_buf(),
        config.static_dir.as_ref().map(|path| path.join("audio")),
    );
    let runtime = Runtime::new(library, Arc::new(engine), url);
    runtime.initialize_documentation(&config.docs_root);
    runtime.initialize_ui_language(ui_preferences.language);
    watch_parent(runtime.clone());
    let endpoint = ipc::endpoint(runtime.library.home())?;
    let ipc_listener = ipc::listen(&endpoint)?;
    let discovery = ipc::Discovery {
        runtime: runtime.info.clone(),
        endpoint: endpoint.to_string_lossy().into_owned(),
    };
    let temp = run.join(format!("runtime-{}.tmp", runtime.info.instance_id));
    std::fs::write(&temp, serde_json::to_vec(&discovery).map_err(storage)?).map_err(storage)?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&temp)
        .and_then(|f| f.sync_all())
        .map_err(storage)?;
    let discovery_path = run.join("runtime.json");
    #[cfg(windows)]
    if discovery_path.exists() {
        std::fs::remove_file(&discovery_path).map_err(storage)?;
    }
    std::fs::rename(&temp, &discovery_path).map_err(storage)?;
    crate::startup_note(format!(
        "Ossia runtime {} at {}",
        runtime.info.instance_id, runtime.info.web_base_url
    ));
    let scheduler = tokio::spawn(runtime.clone().schedule());
    crate::starter::schedule(runtime.clone());
    let maintenance = tokio::spawn(runtime.clone().maintain_storage());
    let watch_runtime = runtime.clone();
    let watcher = tokio::spawn(async move {
        let result = watch_runtime.clone().watch().await;
        if let Err(error) = result {
            watch_runtime.emit("watch-error", None, None, json!({"error":error}));
        }
    });
    let ipc_runtime = runtime.clone();
    let ipc_task = tokio::spawn(async move { ipc::serve(ipc_runtime, ipc_listener).await });
    let web = crate::http::router(
        runtime.clone(),
        config.static_dir,
        config.dev_origin,
        access.allowed_hosts,
    );
    let stop_runtime = runtime.clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let shutdown = tokio::spawn(async move {
        let signal = shutdown_signal();
        tokio::pin!(signal);
        loop {
            if stop_runtime
                .stopping
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            tokio::select! {
                _ = &mut signal => { let _ = stop_runtime.stop(); break; }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        }
        let _ = shutdown_tx.send(true);
    });
    let mut shutdown4 = shutdown_rx.clone();
    let http6 = listener6.map(|listener| {
        let mut shutdown6 = shutdown_rx;
        let web = web.clone();
        tokio::spawn(async move {
            axum::serve(listener, web)
                .with_graceful_shutdown(async move {
                    let _ = shutdown6.wait_for(|stopping| *stopping).await;
                })
                .await
        })
    });
    let http_result = axum::serve(listener, web)
        .with_graceful_shutdown(async move {
            let _ = shutdown4.wait_for(|stopping| *stopping).await;
        })
        .await;
    let _ = runtime.stop();
    let _ = shutdown.await;
    if let Some(http6) = http6 {
        http6.await.map_err(storage)?.map_err(storage)?;
    }
    let _ = scheduler.await;
    let _ = maintenance.await;
    let _ = watcher.await;
    let ipc_result = ipc_task.await.map_err(storage)?;
    if ipc::read_discovery(runtime.library.home())
        .is_ok_and(|d| d.runtime.instance_id == runtime.info.instance_id)
    {
        let _ = std::fs::remove_file(discovery_path);
    }
    http_result.map_err(storage)?;
    ipc_result
}

pub async fn ensure_runtime(config: &Config, executable: &Path) -> Result<ipc::Discovery> {
    ensure_runtime_inner(config, executable, None).await
}

/// Start a runtime owned by the desktop process and stop it when that process disappears.
pub async fn ensure_runtime_owned(
    config: &Config,
    executable: &Path,
    parent_pid: u32,
) -> Result<ipc::Discovery> {
    ensure_runtime_inner(config, executable, Some(parent_pid)).await
}

async fn ensure_runtime_inner(
    config: &Config,
    executable: &Path,
    parent_pid: Option<u32>,
) -> Result<ipc::Discovery> {
    match ipc::connect(&config.home).await {
        Ok(info) => {
            watch_existing_parent(&info, parent_pid).await?;
            return Ok(info);
        }
        Err(error) if error.code == ErrorCode::RuntimeIncompatible => return Err(error),
        _ => {}
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let startup_path = tempfile::Builder::new()
            .prefix("ossia-startup-")
            .tempfile()
            .map_err(storage)?
            .into_temp_path();
        let mut args = vec![
            "--home".into(),
            config.home.as_os_str().to_owned(),
            "--port".into(),
            OsString::from(config.port.to_string()),
            "--engine-root".into(),
            config.engine_root.as_os_str().to_owned(),
            "--docs-root".into(),
            config.docs_root.as_os_str().to_owned(),
        ];
        if let Some(dir) = &config.static_dir {
            args.push("--static-dir".into());
            args.push(dir.as_os_str().to_owned());
        }
        if let Some(origin) = &config.dev_origin {
            args.push("--dev-origin".into());
            args.push(origin.into());
        }
        if let Some(listen) = config.listen {
            args.push("--listen".into());
            args.push(listen.to_string().into());
        }
        if let Some(public_url) = &config.public_url {
            args.push("--public-url".into());
            args.push(public_url.into());
        }
        args.push("--startup-log".into());
        args.push(startup_path.as_os_str().to_owned());
        if let Some(pid) = parent_pid {
            args.push("--parent-pid".into());
            args.push(pid.to_string().into());
        }
        args.push("serve".into());
        let mut child = crate::runtime_spawn::spawn(executable, &args)?;
        let started = Instant::now();
        let mut exit = None;
        let mut busy = false;
        loop {
            match ipc::connect(&config.home).await {
                Ok(info) => {
                    watch_existing_parent(&info, parent_pid).await?;
                    child.detach();
                    return Ok(info);
                }
                Err(error) if error.code == ErrorCode::RuntimeIncompatible => return Err(error),
                _ => {}
            }
            if exit.is_none() {
                exit = child.try_wait().map_err(storage)?;
            }
            if exit.is_some() {
                // A losing concurrent launcher may exit while the lock winner is still starting.
                // If no winner appears, retry after the prior owner releases the library lock.
                let log = std::fs::read_to_string(startup_path.as_os_str()).unwrap_or_default();
                if let Some(error) = log
                    .lines()
                    .filter_map(|line| {
                        serde_json::from_str::<RpcResponse>(line)
                            .ok()
                            .and_then(|r| r.error)
                    })
                    .next_back()
                {
                    if error.code != ErrorCode::LibraryBusy {
                        return Err(error);
                    }
                    busy = true;
                }
                break;
            }
            if started.elapsed() > Duration::from_secs(15) {
                return Err(AppError::new(ErrorCode::RuntimeUnavailable,format!("runtime failed to start or another process holds the library; child exit: {exit:?}")));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if !busy || Instant::now() >= deadline {
            return Err(AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("runtime failed to start; child exit: {exit:?}"),
            ));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn watch_parent(runtime: Arc<Runtime>) {
    let pid = PARENT_PID.load(std::sync::atomic::Ordering::Acquire);
    let pid = (pid != 0).then_some(pid).or_else(|| {
        std::env::var("OSSIA_PARENT_PID")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
    });
    if let Some(pid) = pid {
        runtime.set_parent_pid(pid);
    }
    tokio::spawn(async move {
        while !runtime.stopping.load(std::sync::atomic::Ordering::Acquire) {
            if runtime.parent_pid().is_some_and(|pid| !process_alive(pid)) {
                let _ = runtime.stop();
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

async fn watch_existing_parent(discovery: &ipc::Discovery, parent_pid: Option<u32>) -> Result<()> {
    if let Some(pid) = parent_pid {
        ipc::call(&discovery.endpoint, "watch-parent", json!({"pid": pid})).await?;
    }
    Ok(())
}

#[cfg(unix)]
fn process_alive(pid: u32) -> bool {
    if pid == 0 || pid > libc::pid_t::MAX as u32 {
        return false;
    }
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_TIMEOUT},
        Storage::FileSystem::SYNCHRONIZE,
        System::Threading::{OpenProcess, WaitForSingleObject},
    };
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return false;
        }
        let alive = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
        CloseHandle(handle);
        alive
    }
}

#[cfg(not(any(unix, windows)))]
fn process_alive(_: u32) -> bool {
    true
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! { _=tokio::signal::ctrl_c()=>{},_=signal.recv()=>{} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

/// Inspects configuration and resources without starting a service or opening the library.
pub async fn doctor(config: &Config) -> Result<DoctorReport> {
    let root = config.engine_root.clone();
    let resource = tokio::task::spawn_blocking(move || {
        let scratch = tempfile::tempdir().map_err(storage)?;
        ossia_engine::Engine::verify(&root, scratch.path(), &ossia_engine::RunContext::default())
            .map(|engine| engine.build_id().to_string())
    })
    .await
    .map_err(storage)?;
    let discovery = ipc::connect(&config.home).await;
    Ok(DoctorReport {
        documentation: Some(ossia_library::Documentation::inspect(&config.docs_root)),
        home: path_text(&config.home),
        engine: match resource {
            Ok(id) => EngineHealth {
                ok: true,
                build_id: Some(id),
                error: None,
            },
            Err(error) => EngineHealth {
                ok: false,
                build_id: None,
                error: Some(error),
            },
        },
        runtime: match discovery {
            Ok(d) => RuntimeHealth {
                ok: true,
                info: Some(d.runtime),
                error: None,
            },
            Err(error) => RuntimeHealth {
                ok: false,
                info: None,
                error: Some(error),
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        advertisable_lan_ipv4, configured_listen, network_interface_rank, process_alive,
        resolve_web_access, Ipv4Addr,
    };

    #[test]
    fn parent_liveness_distinguishes_current_and_impossible_pids() {
        assert!(process_alive(std::process::id()));
        assert!(!process_alive(u32::MAX));
    }

    #[test]
    fn lan_access_prefers_the_first_private_address() {
        let access = resolve_web_access(
            None,
            true,
            None,
            &[Ipv4Addr::new(192, 168, 1, 20), Ipv4Addr::new(10, 0, 0, 4)],
            3211,
        )
        .unwrap();
        assert_eq!(configured_listen(None, true), Ipv4Addr::UNSPECIFIED);
        assert_eq!(access.base_url, "http://192.168.1.20:3211");
        assert!(access.allowed_hosts.contains(&"192.168.1.20".into()));
        assert!(access.allowed_hosts.contains(&"10.0.0.4".into()));
    }

    #[test]
    fn physical_interfaces_rank_before_vpn_interfaces() {
        assert!(
            network_interface_rank("en0", Ipv4Addr::new(192, 168, 1, 20))
                < network_interface_rank("utun4", Ipv4Addr::new(10, 0, 0, 4))
        );
        assert!(
            network_interface_rank("Wi-Fi", Ipv4Addr::new(192, 168, 1, 20))
                < network_interface_rank("bridge100", Ipv4Addr::new(192, 168, 64, 1))
        );
    }

    #[test]
    fn lan_access_skips_link_local_addresses() {
        let link_local = Ipv4Addr::new(169, 254, 149, 186);
        let private = Ipv4Addr::new(192, 168, 1, 20);
        assert!(!advertisable_lan_ipv4(link_local));

        let access = resolve_web_access(None, true, None, &[link_local, private], 3211).unwrap();
        assert_eq!(access.base_url, "http://192.168.1.20:3211");
        assert!(!access.allowed_hosts.contains(&link_local.to_string()));
    }

    #[test]
    fn lan_access_without_a_lan_address_uses_loopback() {
        assert_eq!(
            resolve_web_access(None, true, None, &[], 3211)
                .unwrap()
                .base_url,
            "http://localhost:3211"
        );
        assert_eq!(
            resolve_web_access(None, true, None, &[Ipv4Addr::new(169, 254, 149, 186)], 3211,)
                .unwrap()
                .base_url,
            "http://localhost:3211"
        );
    }

    #[test]
    fn invalid_public_urls_fail() {
        assert!(resolve_web_access(
            Some(Ipv4Addr::LOCALHOST),
            false,
            Some("http://example.test:3211"),
            &[],
            3211,
        )
        .is_err());
        assert!(resolve_web_access(
            Some(Ipv4Addr::UNSPECIFIED),
            true,
            Some("http://example.test:9"),
            &[Ipv4Addr::new(192, 168, 1, 20)],
            3211,
        )
        .is_err());
    }
}
