#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let mut decoder = canopybus::StreamDecoder::new();
    let mut pos = 0usize;
    while pos < data.len() {
        let step = (data[pos] as usize % 41).saturating_add(1);
        let end = (pos + step).min(data.len());
        let _ = decoder.push(&data[pos..end]);
        pos = end;
    }
    let _ = decoder.finish();
});
