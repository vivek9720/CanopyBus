pub mod calibration;
pub mod catalog;
pub mod codec;
pub mod envelope;
pub mod error;
pub mod journal;
pub mod manifest;
pub mod model;
pub mod page;
pub mod query;
pub mod reader;
pub mod rules;
pub mod schedule;
pub mod session;
pub mod store;
pub mod text;
pub mod topology;
pub mod validate;

pub use envelope::{parse_archive, parse_binary_archive, parse_stream_archive};
pub use error::{DecodeError, Result};
pub use model::{CanopyArchive, Device, JournalEntry, Link, RuleSet, SamplePage, ScheduleWindow};
pub use session::{FrameOutcome, StreamDecoder};

pub fn parse(data: &[u8]) -> Result<CanopyArchive> {
    parse_archive(data)
}

pub fn decode_and_summarize(data: &[u8]) -> Result<query::ArchiveSummary> {
    let archive = parse_archive(data)?;
    validate::validate_archive(&archive)?;
    let mut planner = query::QueryPlanner::new();
    Ok(planner.summarize(&archive))
}
