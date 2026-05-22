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

pub fn run() {
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
            WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Cinny")
                .on_new_window(move |url, _features| {
                    let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
