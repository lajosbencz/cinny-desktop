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
fn describe_permission_request(request: &webkit2gtk::PermissionRequest) -> Option<String> {
    use webkit2gtk::glib::object::Cast;
    use webkit2gtk::{
        DeviceInfoPermissionRequest, GeolocationPermissionRequest,
        NotificationPermissionRequest, PointerLockPermissionRequest,
        UserMediaPermissionRequest, UserMediaPermissionRequestExt,
    };

    if let Some(umr) = request.dynamic_cast_ref::<UserMediaPermissionRequest>() {
        return Some(match (umr.is_for_audio_device(), umr.is_for_video_device()) {
            (true, true) => "use your microphone and camera".into(),
            (true, false) => "use your microphone".into(),
            (false, true) => "use your camera".into(),
            (false, false) => "use a media device".into(),
        });
    }
    if request.dynamic_cast_ref::<NotificationPermissionRequest>().is_some() {
        return Some("show desktop notifications".into());
    }
    if request.dynamic_cast_ref::<GeolocationPermissionRequest>().is_some() {
        return Some("access your location".into());
    }
    if request.dynamic_cast_ref::<PointerLockPermissionRequest>().is_some() {
        return Some("lock your mouse pointer".into());
    }
    if request.dynamic_cast_ref::<DeviceInfoPermissionRequest>().is_some() {
        return Some("see your media device list".into());
    }
    None
}

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
            #[allow(unused_mut)]
            let mut builder = WebviewWindowBuilder::new(app, "main".to_string(), window_url)
                .title("Cinny")
                .on_new_window(move |url, _features| {
                    let _ = app_handle.opener().open_url(url.as_str(), None::<&str>);
                    NewWindowResponse::Deny
                });

            // WebKitGTK applies setting changes only on the next page load; the
            // webview starts loading before with_webview() runs, so the SPA's
            // first feature-detection misses RTCPeerConnection. Inject a tiny
            // script that triggers a one-shot reload if WebRTC isn't visible
            // yet — by the second load our settings are in effect.
            #[cfg(target_os = "linux")]
            {
                builder = builder.initialization_script(
                    "if (typeof RTCPeerConnection === 'undefined' \
                       && !sessionStorage.getItem('cinny-desktop:webrtc-reload')) { \
                         sessionStorage.setItem('cinny-desktop:webrtc-reload', '1'); \
                         location.reload(); \
                       }",
                );
            }

            let window = builder.build()?;

            // WebKitGTK ships WebRTC but Tauri leaves it off by default. Flip
            // the settings and prompt the user for each permission request the
            // webview makes (mic, camera, notifications, ...).
            // Precedent: https://github.com/tauri-apps/tauri/discussions/8426
            #[cfg(target_os = "linux")]
            {
                use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
                use webkit2gtk::{PermissionRequestExt, SettingsExt, WebViewExt};

                let dialog_handle = app.handle().clone();
                window.with_webview(move |webview| {
                    let wv = webview.inner();
                    if let Some(settings) = WebViewExt::settings(&wv) {
                        settings.set_enable_webrtc(true);
                        settings.set_enable_media_stream(true);
                        settings.set_enable_mediasource(true);
                        settings.set_media_playback_requires_user_gesture(false);
                        settings.set_media_playback_allows_inline(true);
                    }
                    wv.connect_permission_request(move |_, request| {
                        let Some(label) = describe_permission_request(request) else {
                            // Unknown permission type: deny by default.
                            request.deny();
                            return true;
                        };
                        let request = request.clone();
                        dialog_handle
                            .dialog()
                            .message(format!("Cinny wants to {label}."))
                            .title("Permission request")
                            .kind(MessageDialogKind::Info)
                            .buttons(MessageDialogButtons::OkCancelCustom(
                                "Allow".into(),
                                "Deny".into(),
                            ))
                            .show(move |allowed| {
                                if allowed {
                                    request.allow();
                                } else {
                                    request.deny();
                                }
                            });
                        true
                    });
                })?;
            }

            Ok(())
        })
        .run(context)
        .expect("error while building tauri application");
}
