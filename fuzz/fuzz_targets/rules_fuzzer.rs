#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let mut archive = canopybus::CanopyArchive::new(1);
    archive.rules = canopybus::rules::parse_rules_segment(data).unwrap_or_default();
    if archive.rules.is_empty() {
        archive.rules.push(canopybus::RuleSet {
            name: "raw".to_string(),
            bytecode: data.to_vec(),
            labels: Vec::new(),
        });
    }
    let _ = canopybus::rules::evaluate_rules(&archive);
    let _ = canopybus::parse_archive(data)
        .and_then(|archive| canopybus::rules::evaluate_rules(&archive).map(|_| archive));
});
