use r5_battery_estimator::{battery::transport::R5HidTransport, probe_once};

fn main() {
    let result = probe_once(&R5HidTransport::new());
    println!("{}", serde_json::to_string(&result).expect("probe result must serialize"));
    if matches!(result, r5_battery_estimator::ProbeResult::Error { .. }) {
        std::process::exit(2);
    }
}
