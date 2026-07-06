#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(archive) = canopybus::parse_archive(data) {
        let _ = canopybus::validate::validate_archive(&archive);
        let mut planner = canopybus::query::QueryPlanner::new();
        let summary = planner.summarize(&archive);
        let _ = summary.fingerprint ^ planner.last_fingerprint();
        let _ = canopybus::rules::evaluate_rules(&archive);
    }
});
