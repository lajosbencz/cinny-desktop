use tauri::AppHandle;

#[cfg(target_os = "linux")]
mod portal {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tauri::{AppHandle, Emitter};
    use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
    use zbus::{proxy, Connection};

    #[proxy(
        interface = "org.freedesktop.portal.GlobalShortcuts",
        default_service = "org.freedesktop.portal.Desktop",
        default_path = "/org/freedesktop/portal/desktop"
    )]
    trait GlobalShortcuts {
        fn create_session(
            &self,
            options: HashMap<&str, Value<'_>>,
        ) -> zbus::Result<OwnedObjectPath>;

        fn bind_shortcuts(
            &self,
            session_handle: &zbus::zvariant::ObjectPath<'_>,
            shortcuts: Vec<(
                &str,
                HashMap<&str, Value<'_>>,
            )>,
            parent_window: &str,
            options: HashMap<&str, Value<'_>>,
        ) -> zbus::Result<OwnedObjectPath>;

        #[zbus(signal)]
        fn activated(
            &self,
            session_handle: OwnedObjectPath,
            shortcut_id: String,
            timestamp: u64,
            options: HashMap<String, OwnedValue>,
        ) -> zbus::Result<()>;

        #[zbus(signal)]
        fn deactivated(
            &self,
            session_handle: OwnedObjectPath,
            shortcut_id: String,
            timestamp: u64,
            options: HashMap<String, OwnedValue>,
        ) -> zbus::Result<()>;
    }

    pub struct WaylandPttSession {
        _connection: Connection,
        _session_path: Option<OwnedObjectPath>,
    }

    pub static WAYLAND_SESSION: Mutex<Option<WaylandPttSession>> = Mutex::new(None);

    pub async fn start_session(app: AppHandle) -> Result<(), String> {
        let connection = Connection::session()
            .await
            .map_err(|e| format!("Failed to connect to D-Bus session bus: {}", e))?;

        let proxy = GlobalShortcutsProxy::new(&connection)
            .await
            .map_err(|e| format!("Failed to create GlobalShortcuts proxy: {}", e))?;

        let mut session_options: HashMap<&str, Value<'_>> = HashMap::new();
        session_options.insert(
            "handle_token",
            Value::from("cinny_ptt_session"),
        );
        session_options.insert(
            "session_handle_token",
            Value::from("cinny_ptt"),
        );

        let session_path = proxy
            .create_session(session_options)
            .await
            .map_err(|e| format!("Failed to create GlobalShortcuts session: {}", e))?;

        let session_obj_path = zbus::zvariant::ObjectPath::try_from(session_path.as_str())
            .map_err(|e| format!("Invalid session path: {}", e))?;

        let mut shortcut_props: HashMap<&str, Value<'_>> = HashMap::new();
        shortcut_props.insert("description", Value::from("Push to Talk"));
        shortcut_props.insert(
            "preferred_trigger",
            Value::from(""),
        );

        let shortcuts = vec![("push-to-talk", shortcut_props)];
        let bind_options: HashMap<&str, Value<'_>> = HashMap::new();

        proxy
            .bind_shortcuts(&session_obj_path, shortcuts, "", bind_options)
            .await
            .map_err(|e| format!("Failed to bind shortcuts: {}", e))?;

        let app_activated = app.clone();
        let mut activated_stream = proxy
            .receive_activated()
            .await
            .map_err(|e| format!("Failed to listen for Activated signal: {}", e))?;

        let app_deactivated = app.clone();
        let mut deactivated_stream = proxy
            .receive_deactivated()
            .await
            .map_err(|e| format!("Failed to listen for Deactivated signal: {}", e))?;

        tokio::spawn(async move {
            use futures_util::StreamExt;
            while let Some(_signal) = activated_stream.next().await {
                let _ = app_activated.emit("ptt-state", true);
            }
        });

        tokio::spawn(async move {
            use futures_util::StreamExt;
            while let Some(_signal) = deactivated_stream.next().await {
                let _ = app_deactivated.emit("ptt-state", false);
            }
        });

        *WAYLAND_SESSION.lock().unwrap() = Some(WaylandPttSession {
            _connection: connection,
            _session_path: Some(session_path),
        });

        Ok(())
    }

    pub async fn stop_session() -> Result<(), String> {
        let mut session = WAYLAND_SESSION.lock().unwrap();
        if let Some(ref mut s) = *session {
            s._session_path = None;
        }
        *session = None;
        Ok(())
    }
}

#[tauri::command]
pub fn is_wayland() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false)
            || std::env::var("WAYLAND_DISPLAY").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[tauri::command]
pub async fn start_wayland_ptt(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        portal::start_session(app).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Err("Wayland PTT is only available on Linux".to_string())
    }
}

#[tauri::command]
pub async fn stop_wayland_ptt() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        portal::stop_session().await
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}
