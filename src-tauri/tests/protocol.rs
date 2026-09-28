use r5_battery_estimator::battery::protocol::{
    BATTERY_REQUEST, ProtocolError, ReportLayout, parse_battery_report,
};

fn shifted(percent: u8, charging: bool) -> [u8; 65] {
    let mut bytes = [0_u8; 65];
    bytes[1] = 0xa1;
    bytes[4] = 0x02;
    bytes[6] = 0x83;
    bytes[7] = u8::from(charging);
    bytes[8] = percent;
    bytes
}

fn normal(percent: u8, charging: bool) -> [u8; 65] {
    let mut bytes = [0_u8; 65];
    bytes[0] = 0xa1;
    bytes[3] = 0x02;
    bytes[5] = 0x83;
    bytes[6] = u8::from(charging);
    bytes[7] = percent;
    bytes
}

#[test]
fn request_is_the_validated_65_byte_transaction() {
    assert_eq!(BATTERY_REQUEST.len(), 65);
    assert_eq!(BATTERY_REQUEST[3], 0x02);
    assert_eq!(BATTERY_REQUEST[4], 0x02);
    assert_eq!(BATTERY_REQUEST[6], 0x83);
    assert_eq!(BATTERY_REQUEST.iter().filter(|&&byte| byte != 0).count(), 3);
}

#[test]
fn parses_both_captured_layouts_and_boundaries() {
    let zero = parse_battery_report(&shifted(0, false)).unwrap();
    assert_eq!(zero.percent, 0);
    assert_eq!(zero.layout, ReportLayout::Shifted);
    let full = parse_battery_report(&normal(100, true)).unwrap();
    assert_eq!(full.percent, 100);
    assert!(full.charging);
    assert_eq!(full.layout, ReportLayout::Normal);
}

#[test]
fn rejects_out_of_range_truncated_and_wrong_markers() {
    assert_eq!(
        parse_battery_report(&normal(101, false)),
        Err(ProtocolError::InvalidPercent(101))
    );
    assert_eq!(parse_battery_report(&[0; 8]), Err(ProtocolError::Truncated(8)));
    assert_eq!(parse_battery_report(&[0; 65]), Err(ProtocolError::MarkerMismatch));
}

