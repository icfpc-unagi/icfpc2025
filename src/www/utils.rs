use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_humanize::HumanTime;

/// Returns datetime string suffixed with human readable time
/// or the original string if it cannot be parsed.
/// E.g. 2023-07-10 00:13:29 (7 minutes ago)
pub fn maybe_enrich_datetime_str(datetime_str: String) -> String {
    // Parse submission_created that is either of following format:
    // - 2023-07-09T22:40:40.142056715Z (from API)
    // - 2020-01-01 00:00:00 (from MySQL)
    if let Some(parsed) = DateTime::from_str(&datetime_str)
        .ok()
        .or_else(|| DateTime::parse_from_rfc3339(&datetime_str).ok())
        .map(|dt| dt.naive_utc())
        .or_else(|| NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S").ok())
    {
        let human_time = HumanTime::from(Utc::now().naive_utc() - parsed).to_text_en(
            chrono_humanize::Accuracy::Rough,
            chrono_humanize::Tense::Past,
        );
        format!("{} ({})", datetime_str, human_time)
    } else {
        datetime_str
    }
}
