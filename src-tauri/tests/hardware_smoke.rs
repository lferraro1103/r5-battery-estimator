use r5_battery_estimator::battery::transport::{HidTransport, R5HidTransport};

#[test]
#[ignore = "requires the physical R5 Ultra receiver and explicit hardware acceptance"]
fn real_r5_returns_a_bounded_reading() {
    let reading = R5HidTransport::new().query().expect("real R5 query failed");
    assert!(reading.percent <= 100);
}

