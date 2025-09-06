//! # Web Server Utilities
//!
//! This module provides miscellaneous helper functions for the web server,
//! such as formatting data for display in the UI.

use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_humanize::HumanTime;

/// Enriches a datetime string with a human-readable relative time.
///
/// This function attempts to parse a string that represents a UTC datetime.
/// If successful, it appends a human-friendly "time ago" string (e.g., "7 minutes ago").
/// If the string cannot be parsed, it is returned unchanged.
///
/// It supports multiple common datetime formats:
/// - RFC 3339 with nanoseconds and 'Z' (e.g., "2023-07-09T22:40:40.142056715Z")
/// - MySQL's `DATETIME` format (e.g., "2020-01-01 00:00:00")
///
/// # Arguments
/// * `datetime_str` - A string that may contain a datetime.
///
/// # Returns
/// A new string with the human-readable time appended, or the original string.
pub fn maybe_enrich_datetime_str(datetime_str: String) -> String {
    // Attempt to parse the datetime string from a few common formats.
    if let Some(parsed) = DateTime::from_str(&datetime_str)
        .ok()
        .or_else(|| DateTime::parse_from_rfc3339(&datetime_str).ok())
        .map(|dt| dt.naive_utc())
        .or_else(|| NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S").ok())
    {
        // If parsing succeeds, calculate the duration from now and format it.
        let human_time = HumanTime::from(Utc::now().naive_utc() - parsed).to_text_en(
            chrono_humanize::Accuracy::Rough,
            chrono_humanize::Tense::Past,
        );
        format!("{} ({})", datetime_str, human_time)
    } else {
        // If parsing fails, return the original string.
        datetime_str
    }
}
