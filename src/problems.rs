use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Problem {
    pub problem_name: &'static str,
    pub size: usize,
}

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
];

pub fn all_problems() -> &'static [Problem] {
    PROBLEMS_DATA
}

// Build a name -> problem map once for O(1) lookup.
static PROBLEM_MAP: Lazy<HashMap<&'static str, &'static Problem>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for p in PROBLEMS_DATA.iter() {
        m.insert(p.problem_name, p);
    }
    m
});

pub fn get_problem(name: &str) -> Option<&'static Problem> {
    PROBLEM_MAP.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_problems_contains_expected_entries() {
        let all = all_problems();
        assert_eq!(all.len(), 6);
        let names: Vec<&str> = all.iter().map(|p| p.problem_name).collect();
        assert_eq!(
            names,
            vec![
                "probatio", "primus", "secundus", "tertius", "quartus", "quintus"
            ]
        );
        let sizes: Vec<usize> = all.iter().map(|p| p.size).collect();
        assert_eq!(sizes, vec![3, 6, 12, 18, 24, 30]);
    }

    #[test]
    fn get_problem_returns_expected() {
        let p = get_problem("quintus").expect("quintus should exist");
        assert_eq!(p.size, 30);
        assert!(get_problem("unknown").is_none());
    }
}
