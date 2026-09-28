use std::{sync::{Arc, Mutex}, thread, time::Duration};

use r5_battery_estimator::{ProbeResult, battery::transport::R5HidTransport, probe_once};
use tauri::{Manager, WindowEvent, menu::{Menu, MenuItem}, tray::TrayIconBuilder};

#[derive(Clone)]
struct BatteryState(Arc<Mutex<ProbeResult>>);

#[tauri::command]
fn current_battery(state: tauri::State<'_, BatteryState>) -> ProbeResult {
    state.0.lock().expect("battery state poisoned").clone()
}

#[tauri::command]
fn refresh_battery(state: tauri::State<'_, BatteryState>) -> ProbeResult {
    let result = probe_once(&R5HidTransport::new());
    *state.0.lock().expect("battery state poisoned") = result.clone();
    result
}

fn poll_in_background(state: BatteryState) {
    thread::spawn(move || loop {
        let result = probe_once(&R5HidTransport::new());
        *state.0.lock().expect("battery state poisoned") = result;
        thread::sleep(Duration::from_secs(30));
    });
}

fn main() {
    if std::env::args().any(|arg| arg == "--probe-once") {
        let result = probe_once(&R5HidTransport::new());
        println!(
            "{}",
            serde_json::to_string(&result).expect("probe result must serialize")
        );
        if matches!(result, r5_battery_estimator::ProbeResult::Error { .. }) {
            std::process::exit(2);
        }
        return;
    }

    let state = BatteryState(Arc::new(Mutex::new(probe_once(&R5HidTransport::new()))));
    poll_in_background(state.clone());

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Abrir panel", true, None::<&str>)?;
            let update = MenuItem::with_id(app, "update", "Actualizar batería", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &update, &quit])?;
            TrayIconBuilder::with_id("main")
                .menu(&menu)
                .tooltip("R5 Battery Estimator")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => { if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } }
                    "update" => { let state = app.state::<BatteryState>(); let result = probe_once(&R5HidTransport::new()); *state.0.lock().expect("battery state poisoned") = result; }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![current_battery, refresh_battery])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run R5 Battery Estimator");
}

