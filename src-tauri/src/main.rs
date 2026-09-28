use r5_battery_estimator::{battery::transport::R5HidTransport, probe_once};

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

    eprintln!("R5 Battery Estimator: use --probe-once until the desktop shell is built");
}

