use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use hidapi::HidApi;
use thiserror::Error;

use super::protocol::{
    BATTERY_REQUEST, ParsedBattery, ProtocolError, REPORT_LENGTH, R5_PRODUCT_ID, R5_VENDOR_ID,
    parse_battery_report,
};

pub const DEVICE_SETTLE_DELAY: Duration = Duration::from_millis(120);
pub const RESPONSE_BUDGET: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("R5 Ultra vendor HID interface is not present")]
    DeviceUnavailable,
    #[error("another feature transaction still owns the HID lane")]
    Busy,
    #[error("feature transaction exceeded its {0} ms response budget")]
    Timeout(u64),
    #[error("HID operation failed: {0}")]
    Hid(String),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("feature transaction thread ended without a result")]
    WorkerStopped,
}

pub trait HidTransport: Send + Sync {
    fn query(&self) -> Result<ParsedBattery, TransportError>;
}

#[derive(Clone, Default)]
pub struct R5HidTransport {
    lane_owned: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
}

impl R5HidTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn query_with_budget(&self, budget: Duration) -> Result<ParsedBattery, TransportError> {
        if self
            .lane_owned
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(TransportError::Busy);
        }

        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let lane_owned = Arc::clone(&self.lane_owned);
        let current_generation = Arc::clone(&self.generation);
        let (sender, receiver) = mpsc::sync_channel(1);

        thread::spawn(move || {
            let result = query_device_once();
            lane_owned.store(false, Ordering::Release);
            if current_generation.load(Ordering::Acquire) == generation {
                let _ = sender.send(result);
            }
        });

        match receiver.recv_timeout(budget) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Invalidate this generation so a late result cannot be published.
                self.generation.fetch_add(1, Ordering::AcqRel);
                Err(TransportError::Timeout(budget.as_millis() as u64))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(TransportError::WorkerStopped),
        }
    }
}

impl HidTransport for R5HidTransport {
    fn query(&self) -> Result<ParsedBattery, TransportError> {
        self.query_with_budget(RESPONSE_BUDGET)
    }
}

fn query_device_once() -> Result<ParsedBattery, TransportError> {
    let api = HidApi::new().map_err(|error| TransportError::Hid(error.to_string()))?;
    let candidates: Vec<_> = api
        .device_list()
        .filter(|device| {
            device.vendor_id() == R5_VENDOR_ID
                && device.product_id() == R5_PRODUCT_ID
                && (device.usage_page() >= 0xff00 || device.interface_number() == 2)
        })
        .collect();

    if candidates.is_empty() {
        return Err(TransportError::DeviceUnavailable);
    }

    let mut errors = Vec::new();
    for candidate in candidates {
        let attempt = (|| {
            let device = candidate
                .open_device(&api)
                .map_err(|error| TransportError::Hid(error.to_string()))?;
            device
                .send_feature_report(&BATTERY_REQUEST)
                .map_err(|error| TransportError::Hid(error.to_string()))?;
            thread::sleep(DEVICE_SETTLE_DELAY);
            let mut response = [0_u8; REPORT_LENGTH];
            let received = device
                .get_feature_report(&mut response)
                .map_err(|error| TransportError::Hid(error.to_string()))?;
            parse_battery_report(&response[..received]).map_err(TransportError::from)
        })();

        match attempt {
            Ok(reading) => return Ok(reading),
            Err(error) => errors.push(error.to_string()),
        }
    }

    Err(TransportError::Hid(errors.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_returns_without_publishing_late_result_or_overlapping() {
        let transport = R5HidTransport::new();
        transport.lane_owned.store(true, Ordering::Release);
        assert_eq!(transport.query(), Err(TransportError::Busy));
        transport.lane_owned.store(false, Ordering::Release);
    }
}

