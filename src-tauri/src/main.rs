use r5_battery_estimator::{ProbeResult, battery::transport::R5HidTransport, probe_once};

#[tauri::command]
fn current_battery() -> ProbeResult {
    probe_once(&R5HidTransport::new())
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

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![current_battery])
        .run(tauri::generate_context!())
        .expect("failed to run R5 Battery Estimator");
}

