use ossia_backend::{AppError, ErrorCode, Result, UiLanguage};
use std::path::{Path, PathBuf};
use tauri::{
    webview::{DownloadEvent, WebviewWindowBuilder},
    Manager, WebviewUrl, WindowEvent,
};

pub const LABEL: &str = "management";
const SCORE_LABEL_PREFIX: &str = "score-";

fn title(language: UiLanguage) -> &'static str {
    match language {
        UiLanguage::EnUs => "Ossia",
        UiLanguage::ZhCn => "Ossia",
    }
}

fn storage(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::StorageError, error.to_string())
}

pub fn create(
    app: &tauri::AppHandle,
    visible: bool,
    language: UiLanguage,
) -> Result<tauri::WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("main.html".into()))
        .title(title(language))
        .inner_size(1080.0, 720.0)
        .min_inner_size(860.0, 560.0)
        .center()
        .visible(visible)
        .focused(visible);
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .traffic_light_position(tauri::LogicalPosition::new(24.0, 26.0));
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        builder = builder.decorations(false);
    }
    let window = builder.build().map_err(storage)?;
    #[cfg(all(debug_assertions, any(target_os = "macos", target_os = "windows")))]
    {
        use tauri::menu::{Menu, MenuItem, Submenu};

        #[cfg(target_os = "macos")]
        let shortcut = "Cmd+Alt+I";
        #[cfg(target_os = "windows")]
        let shortcut = "Ctrl+Shift+I";
        let item = MenuItem::with_id(app, "devtools", "Developer Tools", true, Some(shortcut))
            .map_err(storage)?;
        let submenu = Submenu::with_items(app, "Developer", true, &[&item]).map_err(storage)?;
        #[cfg(target_os = "macos")]
        let menu = app
            .menu()
            .map(Ok)
            .unwrap_or_else(|| Menu::default(app))
            .map_err(storage)?;
        #[cfg(target_os = "windows")]
        let menu = Menu::new(app).map_err(storage)?;
        menu.append(&submenu).map_err(storage)?;
        #[cfg(target_os = "macos")]
        app.set_menu(menu).map_err(storage)?;
        #[cfg(target_os = "windows")]
        window.set_menu(menu).map_err(storage)?;

        let devtools_window = window.clone();
        window.on_menu_event(move |_, event| {
            if event.id().as_ref() == "devtools" && devtools_window.is_focused().unwrap_or(false) {
                devtools_window.open_devtools();
            }
        });
    }
    let handle = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide(&handle);
        }
    });
    #[cfg(target_os = "macos")]
    app.set_activation_policy(if visible {
        tauri::ActivationPolicy::Regular
    } else {
        tauri::ActivationPolicy::Accessory
    })
    .map_err(storage)?;
    Ok(window)
}

pub fn show(app: &tauri::AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "management window is missing"))?;
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular)
        .map_err(storage)?;
    window.show().map_err(storage)?;
    window.unminimize().map_err(storage)?;
    window.set_focus().map_err(storage)
}

pub fn hide(app: &tauri::AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "management window is missing"))?;
    window.hide().map_err(storage)?;
    #[cfg(target_os = "macos")]
    if !score_window_open(app, None) {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory)
            .map_err(storage)?;
    }
    Ok(())
}

fn score_window_open(app: &tauri::AppHandle, except: Option<&str>) -> bool {
    app.webview_windows().iter().any(|(label, window)| {
        label.starts_with(SCORE_LABEL_PREFIX)
            && Some(label.as_str()) != except
            && window.is_visible().unwrap_or(false)
    })
}

/// Open a score page in its own app window instead of the default browser. Each score gets one
/// window; opening it again brings that window forward.
pub fn open_score(app: &tauri::AppHandle, document_id: &str, url: &str) -> Result<()> {
    let label = format!("{SCORE_LABEL_PREFIX}{document_id}");
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular)
        .map_err(storage)?;
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(storage)?;
        window.unminimize().map_err(storage)?;
        return window.set_focus().map_err(storage);
    }
    let mut url = tauri::Url::parse(url).map_err(storage)?;
    // The assistant's permission only covers loopback score pages; the local server answers there
    // even when LAN access gives it another public address.
    url.set_host(Some("127.0.0.1")).map_err(storage)?;
    // Tells the score page it is in its own window, so it hides its title and score switcher.
    url.query_pairs_mut().append_pair("window", "1");
    let downloads = app.path().download_dir().ok();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title("Ossia")
        .inner_size(1200.0, 860.0)
        .min_inner_size(720.0, 520.0)
        .center()
        .focused(true)
        // The score page sets its document title to the score's title.
        .on_document_title_changed(|window, title| {
            let _ = window.set_title(&title);
        })
        .on_download(move |_, event| {
            match event {
                // Exports are saved to Downloads without overwriting earlier files.
                DownloadEvent::Requested { destination, .. } => {
                    if let Some(downloads) = &downloads {
                        let name = destination
                            .file_name()
                            .map(|name| name.to_owned())
                            .unwrap_or_else(|| "download".into());
                        *destination = unique_path(downloads, Path::new(&name));
                    }
                }
                DownloadEvent::Finished {
                    path: Some(path),
                    success: true,
                    ..
                } => reveal(&path),
                _ => {}
            }
            true
        })
        .build()
        .map_err(storage)?;
    let handle = app.clone();
    let closing = label.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            handle.state::<crate::assistant::Assistant>().stop(&closing);
            #[cfg(target_os = "macos")]
            {
                let management_visible = handle
                    .get_webview_window(LABEL)
                    .and_then(|window| window.is_visible().ok())
                    .unwrap_or(false);
                if !management_visible && !score_window_open(&handle, Some(&closing)) {
                    let _ = handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        }
    });
    Ok(())
}

fn unique_path(directory: &Path, name: &Path) -> PathBuf {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = name.file_stem().and_then(|s| s.to_str()).unwrap_or("download");
    let extension = name.extension().and_then(|s| s.to_str());
    (1..)
        .map(|index| {
            let file = match extension {
                Some(extension) => format!("{stem} ({index}).{extension}"),
                None => format!("{stem} ({index})"),
            };
            directory.join(file)
        })
        .find(|path| !path.exists())
        .unwrap_or(candidate)
}

/// Show a finished export in Finder or Explorer.
fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = path;
}

pub fn set_language(app: &tauri::AppHandle, language: UiLanguage) -> Result<()> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound, "management window is missing"))?;
    window.set_title(title(language)).map_err(storage)
}
