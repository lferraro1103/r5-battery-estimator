use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const R5_VENDOR_ID: u16 = 0x373e;
pub const R5_PRODUCT_ID: u16 = 0x0047;
pub const REPORT_LENGTH: usize = 65;
pub const BATTERY_REQUEST: [u8; REPORT_LENGTH] = {
    let mut report = [0_u8; REPORT_LENGTH];
    report[3] = 0x02;
    report[4] = 0x02;
    report[6] = 0x83;
    report
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportLayout {
    Shifted,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedBattery {
    pub percent: u8,
    pub charging: bool,
    pub layout: ReportLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    #[error("battery report is truncated: expected at least 9 bytes, got {0}")]
    Truncated(usize),
    #[error("receiver has not completed the battery query")]
    Pending,
    #[error("battery report markers do not match a validated R5 layout")]
    MarkerMismatch,
    #[error("battery percentage is outside 0..=100: {0}")]
    InvalidPercent(u8),
}

pub fn parse_battery_report(bytes: &[u8]) -> Result<ParsedBattery, ProtocolError> {
    if bytes.len() < 9 {
        return Err(ProtocolError::Truncated(bytes.len()));
    }

    if (bytes[1] == 0xa0 && bytes[4] == 0x02 && bytes[6] == 0x83)
        || (bytes[0] == 0xa0 && bytes[3] == 0x02 && bytes[5] == 0x83)
    {
        return Err(ProtocolError::Pending);
    }

    let (layout, charging, percent) = if bytes[1] == 0xa1 && bytes[4] == 0x02 && bytes[6] == 0x83 {
        (ReportLayout::Shifted, bytes[7] != 0, bytes[8])
    } else if bytes[0] == 0xa1 && bytes[3] == 0x02 && bytes[5] == 0x83 {
        (ReportLayout::Normal, bytes[6] != 0, bytes[7])
    } else {
        return Err(ProtocolError::MarkerMismatch);
    };

    if percent > 100 {
        return Err(ProtocolError::InvalidPercent(percent));
    }

    Ok(ParsedBattery {
        percent,
        charging,
        layout,
    })
}

