pub mod battery;

use battery::{protocol::ParsedBattery, transport::HidTransport};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProbeResult {
    Ok { reading: ParsedBattery },
    Error { code: &'static str, message: String },
}

pub fn probe_once(transport: &dyn HidTransport) -> ProbeResult {
    match transport.query() {
        Ok(reading) => ProbeResult::Ok { reading },
        Err(error) => ProbeResult::Error {
            code: match error {
                battery::transport::TransportError::DeviceUnavailable => "device_unavailable",
                battery::transport::TransportError::Busy => "busy",
                battery::transport::TransportError::Timeout(_) => "timeout",
                battery::transport::TransportError::Protocol(_) => "malformed_report",
                battery::transport::TransportError::Hid(_) => "hid_error",
                battery::transport::TransportError::WorkerStopped => "worker_stopped",
            },
            message: error.to_string(),
        },
    }
}

