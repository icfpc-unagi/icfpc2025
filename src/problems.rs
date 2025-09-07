//! # Contest Problem Definitions
//!
//! This module contains the static definitions for the official contest problems,
//! including their names and sizes (number of rooms). It provides convenient
//! functions for accessing this data.

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;

/// Represents a single contest problem.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Problem {
    /// The official name of the problem, e.g., "probatio".
    pub problem: String,
    /// The number of rooms in the problem's map.
    pub size: usize,
}

/// A static array containing the data for all known contest problems.
/// Run the following command to update the data:
/// ```bash
///   curl -L https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com/select -o ./src/problems.json
/// ```
static PROBLEMS_DATA: Lazy<Vec<Problem>> = Lazy::new(|| {
    const PROBLEMS_JSON: &str = include_str!("problems.json");
    serde_json::from_str(PROBLEMS_JSON).expect("failed to parse problems.json")
});

/// Returns a slice containing all defined contest problems.
pub fn all_problems() -> &'static [Problem] {
    &PROBLEMS_DATA
}

/// A lazily-initialized HashMap for efficient lookup of problems by name.
/// This avoids iterating through the `PROBLEMS_DATA` slice on every lookup.
static PROBLEM_MAP: Lazy<HashMap<&str, &Problem>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for p in PROBLEMS_DATA.iter() {
        m.insert(p.problem.as_str(), p);
    }
    m
});

/// Looks up a problem by its name.
///
/// # Arguments
/// * `name` - The name of the problem to find.
///
/// # Returns
/// An `Option<&'static Problem>` which is `Some` if a problem with the
/// given name exists, and `None` otherwise.
pub fn get_problem(name: &str) -> Option<&'static Problem> {
    PROBLEM_MAP.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_problems_contains_expected_entries() {
        let all = all_problems();
        assert!(all.len() >= 16);
        let all = all
            .iter()
            .map(|p| (p.problem.as_str(), p.size))
            .collect::<Vec<_>>();
        let expected = [("probatio", 3), ("aleph", 12), ("vau", 18)];
        for (expected_name, expected_size) in expected {
            assert!(
                all.contains(&(expected_name, expected_size)),
                "missing expected problem: {} with size {}",
                expected_name,
                expected_size
            );
        }
    }

    #[test]
    fn get_problem_returns_expected() {
        let p = get_problem("quintus").expect("quintus should exist");
        assert_eq!(p.size, 30);
        assert!(get_problem("unknown").is_none());
    }
}
