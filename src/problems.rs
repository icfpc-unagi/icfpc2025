//! # Contest Problem Definitions
//!
//! This module contains the static definitions for the official contest problems,
//! including their names and sizes (number of rooms). It provides convenient
//! functions for accessing this data.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Represents a single contest problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Problem {
    /// The official name of the problem, e.g., "probatio".
    pub problem_name: &'static str,
    /// The number of rooms in the problem's map.
    pub size: usize,
}

/// A static array containing the data for all known contest problems.
const PROBLEMS_DATA: &[Problem] = &[
    Problem {
        problem_name: "probatio",
        size: 3,
    },
    Problem {
        problem_name: "primus",
        size: 6,
    },
    Problem {
        problem_name: "secundus",
        size: 12,
    },
    Problem {
        problem_name: "tertius",
        size: 18,
    },
    Problem {
        problem_name: "quartus",
        size: 24,
    },
    Problem {
        problem_name: "quintus",
        size: 30,
    },
    Problem {
        problem_name: "aleph",
        size: 12,
    },
    Problem {
        problem_name: "beth",
        size: 24,
    },
    Problem {
        problem_name: "gimel",
        size: 36,
    },
    Problem {
        problem_name: "daleth",
        size: 48,
    },
    Problem {
        problem_name: "he",
        size: 60,
    },
    Problem {
        problem_name: "vau",
        size: 18,
    },
    Problem {
        problem_name: "zain",
        size: 36,
    },
    Problem {
        problem_name: "hhet",
        size: 54,
    },
    Problem {
        problem_name: "teth",
        size: 72,
    },
    Problem {
        problem_name: "iod",
        size: 90,
    },
];

/// Returns a slice containing all defined contest problems.
pub fn all_problems() -> &'static [Problem] {
    PROBLEMS_DATA
}

/// A lazily-initialized HashMap for efficient lookup of problems by name.
/// This avoids iterating through the `PROBLEMS_DATA` slice on every lookup.
static PROBLEM_MAP: Lazy<HashMap<&'static str, &'static Problem>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for p in PROBLEMS_DATA.iter() {
        m.insert(p.problem_name, p);
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
        assert_eq!(all.len(), 16);
        let names: Vec<&str> = all.iter().map(|p| p.problem_name).collect();
        assert_eq!(
            names,
            vec![
                "probatio",
                "primus",
                "secundus",
                "tertius",
                "quartus",
                "quintus",
                "aleph",
                "beth",
                "gimel",
                "daleth",
                "he",
                "vau",
                "zain",
                "hhet",
                "teth",
                "iod",
            ]
        );
        let sizes: Vec<usize> = all.iter().map(|p| p.size).collect();
        assert_eq!(
            sizes,
            vec![3, 6, 12, 18, 24, 30, 12, 24, 36, 48, 60, 18, 36, 54, 72, 90]
        );
    }

    #[test]
    fn get_problem_returns_expected() {
        let p = get_problem("quintus").expect("quintus should exist");
        assert_eq!(p.size, 30);
        assert!(get_problem("unknown").is_none());
    }
}
