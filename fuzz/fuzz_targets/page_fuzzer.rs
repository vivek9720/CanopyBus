#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = canopybus::page::decode_sample_page(data).map(|page| {
        let _ = canopybus::page::page_energy(&page);
        let _ = canopybus::page::resample_page(&page, (data.len() % 9).max(1));
    });
    let _ = canopybus::page::parse_page_segment(data);
});
