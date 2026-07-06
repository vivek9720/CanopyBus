#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = canopybus::journal::parse_journal_segment(data).map(|entries| {
        let _ = canopybus::journal::summarize_journal(&entries);
    });
    if let Ok(archive) = canopybus::parse_archive(data) {
        let _ = canopybus::journal::summarize_journal(&archive.journals);
    }
});
