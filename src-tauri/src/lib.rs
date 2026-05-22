#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

// mod menu;
mod macos_permissions;
mod ptt;
mod wayland_ptt;

use tauri::{webview::{NewWindowResponse, WebviewWindowBuilder}, WebviewUrl};
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "linux")]
fn apply_webkit_workarounds() {
    // Disable WebKitGTK's DMA-BUF renderer; it fails on many host GPU stacks
    // (NVIDIA, certain Wayland compositors, AppImage bundles). Set only if the
    // user hasn't overridden it.
    // Precedent: https://github.com/refactoringhq/tolaria/commit/8c286a4856637d662f05428f679faa4aee607c66
    for (key, value) in [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")] {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

pub fn run() {
    #[cfg(target_os = "linux")]
    apply_webkit_workarounds();

    let port: u16 = 44548;
    let context = tauri::generate_context!();
    let builder = tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always);

    // #[cfg(target_os = "macos")]
    // {
    //     builder = builder.menu(menu::menu());
    // }

    builder
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(ptt::PttState::new())
        .invoke_handler(tauri::generate_handler![
            ptt::ptt_register,
            ptt::ptt_unregister,
            ptt::ptt_start_capture,
            ptt::ptt_stop_capture,
            macos_permissions::check_input_monitoring,
            macos_permissions::request_input_monitoring,
            wayland_ptt::is_wayland,
            wayland_ptt::start_wayland_ptt,
            wayland_ptt::stop_wayland_ptt,
        ])
        .setup(move |app| {
            // Dev: use devUrl from tauri.conf.json (http://localhost:8080) to support HMR
            #[cfg(debug_assertions)]
            let window_url = WebviewUrl::App(Default::default());

            // Release: tauri-plugin-localhost serves bundled frontend assets on this port
            #[cfg(not(debug_assertions))]
            let window_url = {
                let url = format!("http://localhost:{}", port).parse().unwrap();
                WebviewUrl::External(url)
            };

            let app_handle = app.handle().clone();
            let window = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Cinny")
                .on_new_window(move |url, _features| {
                    let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    NewWindowResponse::Deny
                })
                .build()?;

            // WebKitGTK ships WebRTC but Tauri leaves it off by default. Flip
            // the settings and auto-allow getUserMedia / display-capture so
            // Cinny's calling UI can negotiate.
            // Precedent: https://github.com/tauri-apps/tauri/discussions/8426
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};
                window.with_webview(|webview| {
                    let wv = webview.inner();
                    if let Some(settings) = WebViewExt::settings(&wv) {
                        settings.set_enable_webrtc(true);
                        settings.set_enable_media_stream(true);
                        settings.set_enable_mediasource(true);
                        settings.set_media_playback_requires_user_gesture(false);
                        settings.set_media_playback_allows_inline(true);
                    }
                    wv.connect_permission_request(move |_, request| {
                        request.allow();
                        true
                    });
                })?;
            }

            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
