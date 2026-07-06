use canopybus::{codec, parse_archive};

#[test]
fn parses_text_seed_and_writes_summary_text() {
    let input = b"facility west-01 West Bench\ndevice 1 gateway 1 gw basil_seed_000 temp humidity\ndevice 2 climate 1 c1 lettuce_veg_042 temp humidity co2\nlink 1 2 observes 7\nwindow 1 360 720 22 1 morning\nsample 2 temp 1000 21,22 22,23\njournal 1000 boot 1 1 online\n";
    let archive = parse_archive(input).unwrap();
    assert_eq!(archive.manifest.devices.len(), 2);
    let text = codec::write_text_archive(&archive);
    assert!(text.contains("facility west-01"));
}
