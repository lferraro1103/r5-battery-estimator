use std::{
    sync::{
        Arc,
        OnceLock,
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
// 12 reads at 120ms leave room inside the 2s caller budget for enumeration/I/O.
const MAX_RESPONSE_READS: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("R5 Ultra vendor HID interface is not present")]
    DeviceUnavailable,
    #[error("receiver is present but the mouse battery reading is not ready")]
    ReadingUnavailable,
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

#[derive(Clone)]
pub struct R5HidTransport {
    lane_owned: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
}

impl Default for R5HidTransport {
    fn default() -> Self {
        // All independently constructed transports also share the process lane.
        static SHARED: OnceLock<R5HidTransport> = OnceLock::new();
        SHARED.get_or_init(|| Self {
            lane_owned: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
        }).clone()
    }
}

struct LaneLease(Arc<AtomicBool>);
impl Drop for LaneLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl R5HidTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn query_with_budget(&self, budget: Duration) -> Result<ParsedBattery, TransportError> {
        self.query_work_with_budget(budget, query_device_once)
    }

    fn query_work_with_budget<F>(&self, budget: Duration, work: F) -> Result<ParsedBattery, TransportError>
    where F: FnOnce() -> Result<ParsedBattery, TransportError> + Send + 'static {
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
            let lease = LaneLease(lane_owned);
            let result = work();
            if current_generation.load(Ordering::Acquire) == generation {
                let _ = sender.send(result);
            }
            drop(lease);
        });

        match receiver.recv_timeout(budget) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Invalidate this generation so a late result cannot be published.
                let _ = self.generation.compare_exchange(
                    generation, generation + 1, Ordering::AcqRel, Ordering::Acquire,
                );
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
            read_battery_response(|| {
                let mut response = [0_u8; REPORT_LENGTH];
                let received = device.get_feature_report(&mut response)
                    .map_err(|error| TransportError::Hid(error.to_string()))?;
                Ok((response, received))
            }, thread::sleep)
        })();

        match attempt {
            Ok(reading) => return Ok(reading),
            // A responding feature interface owns this reply. Trying unrelated
            // vendor collections only hides pending/protocol state in HID errors.
            Err(error @ (TransportError::ReadingUnavailable | TransportError::Protocol(_))) => {
                return Err(error);
            }
            Err(error) => errors.push(error.to_string()),
        }
    }

    Err(TransportError::Hid(errors.join("; ")))
}

fn read_battery_response<F, W>(mut read: F, mut wait: W) -> Result<ParsedBattery, TransportError>
where
    F: FnMut() -> Result<([u8; REPORT_LENGTH], usize), TransportError>,
    W: FnMut(Duration),
{
    // Match the official retrySetGet path for A0: reread the same request,
    // without resending it or ever publishing the unvalidated zero payload.
    for _ in 0..MAX_RESPONSE_READS {
        wait(DEVICE_SETTLE_DELAY);
        let (response, received) = read()?;
        match parse_battery_report(&response[..received]) {
            Err(ProtocolError::Pending) => continue,
            result => return result.map_err(TransportError::from),
        }
    }
    Err(TransportError::ReadingUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(status: u8, percent: u8) -> ([u8; REPORT_LENGTH], usize) {
        let mut response = BATTERY_REQUEST;
        response[1] = status;
        response[8] = percent;
        (response, REPORT_LENGTH)
    }

    #[test]
    fn pending_response_is_reread_until_validated_battery_arrives() {
        let mut reads = 0;
        let result = read_battery_response(|| {
            reads += 1;
            Ok(frame(if reads == 1 { 0xa0 } else { 0xa1 }, if reads == 1 { 0 } else { 92 }))
        }, |_| {});
        assert_eq!(result.unwrap().percent, 92);
        assert_eq!(reads, 2);
    }

    #[test]
    fn pending_exhaustion_is_unavailable_never_zero_and_within_budget() {
        let mut reads = 0;
        let mut waited = Duration::ZERO;
        let result = read_battery_response(|| {
            reads += 1;
            Ok(frame(0xa0, 0))
        }, |delay| waited += delay);
        assert_eq!(result, Err(TransportError::ReadingUnavailable));
        assert_eq!(reads, MAX_RESPONSE_READS);
        assert!(waited < RESPONSE_BUDGET);
    }

    #[test]
    fn accepts_last_allowed_reply_and_valid_zero_without_retry() {
        for (pending_reads, percent) in [(0, 0), (MAX_RESPONSE_READS - 1, 100)] {
            let mut reads = 0;
            let result = read_battery_response(|| {
                reads += 1;
                Ok(frame(if reads <= pending_reads { 0xa0 } else { 0xa1 }, percent))
            }, |_| {});
            assert_eq!(result.unwrap().percent, percent);
            assert_eq!(reads, pending_reads + 1);
        }
    }

    #[test]
    fn malformed_reply_and_io_error_are_not_retried() {
        let mut reads = 0;
        let result = read_battery_response(|| {
            reads += 1;
            Ok(frame(0xa2, 0))
        }, |_| {});
        assert_eq!(result, Err(TransportError::Protocol(ProtocolError::MarkerMismatch)));
        assert_eq!(reads, 1);
        assert_eq!(read_battery_response(|| Err(TransportError::Hid("I/O".into())), |_| {}),
            Err(TransportError::Hid("I/O".into())));
    }

    #[test]
    fn timeout_returns_without_publishing_late_result_or_overlapping() {
        use super::super::protocol::ReportLayout;
        let transport = R5HidTransport::new();
        let other = R5HidTransport::new();
        assert!(Arc::ptr_eq(&transport.lane_owned, &other.lane_owned));
        let (release, blocked) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let result = transport.query_work_with_budget(Duration::from_millis(20), move || {
            blocked.recv().unwrap();
            finished.send(()).unwrap();
            Ok(ParsedBattery { percent: 99, charging: false, layout: ReportLayout::Normal })
        });
        assert_eq!(result, Err(TransportError::Timeout(20)));
        assert_eq!(other.query_work_with_budget(Duration::from_secs(1), || panic!("overlap")), Err(TransportError::Busy));
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_secs(1)).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while transport.lane_owned.load(Ordering::Acquire) {
            assert!(std::time::Instant::now() < deadline);
            thread::yield_now();
        }
        let next = ParsedBattery { percent: 42, charging: true, layout: ReportLayout::Shifted };
        assert_eq!(other.query_work_with_budget(Duration::from_secs(1), move || Ok(next)), Ok(next), "next request gets its own reading, never the late 99% result");
        assert_eq!(result, Err(TransportError::Timeout(20)), "late completion never replaces timed-out result");
    }
}

