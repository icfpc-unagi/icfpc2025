//! # Random Map Generation
//!
//! This module provides functions for generating random valid Aedificium maps.
//! The generation algorithm ensures that the resulting map corresponds to a
//! 6-regular graph by creating a random perfect matching on the set of all
//! doors across all rooms.

use crate::api;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Generates a random map as a simple vector-based adjacency list.
///
/// This format is likely used for internal testing or solvers that prefer a
/// direct graph representation over the `api::Map` struct.
///
/// # Arguments
/// * `n_rooms` - The number of rooms in the map.
/// * `seed` - An optional seed for the random number generator for reproducibility.
///
/// # Returns
/// A vector where each element is a tuple `(doors, hash)`. `doors` is an array
/// of 6 `usize`s, where `doors[d]` is the index of the room connected to door `d`.
/// `hash` is a simple hash value for the room (signature).
pub fn generate_as_vec(n_rooms: usize, seed: Option<u64>) -> Vec<([usize; 6], u8)> {
    let mut rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_os_rng(),
    };

    // Create a flat list of all doors, identify them by their room index,
    // shuffle them, and then pair them up to create connections.
    let mut doors: Vec<usize> = (0..n_rooms)
        .flat_map(|i| std::iter::repeat_n(i, 6))
        .collect();
    doors.shuffle(&mut rng);
    let conns: Vec<_> = doors
        .chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect();

    // Build an adjacency list from the connection pairs.
    let mut adj = vec![vec![]; n_rooms];
    for &(a, b) in &conns {
        adj[a].push(b);
        adj[b].push(a);
    }

    // Construct the final output format.
    let mut map: Vec<([usize; 6], u8)> = Vec::with_capacity(n_rooms);
    for (i, neighbors) in adj.iter().enumerate() {
        let hash = (i % 4) as u8; // Simple hash based on room index.
        let mut doors = [0; 6];
        doors.copy_from_slice(&neighbors[..6]);
        map.push((doors, hash));
    }
    map
}

/// Generates a random map in the `api::Map` format.
///
/// This format is the one required for submitting a guess to the contest API.
/// The generation logic ensures a valid 6-regular graph by creating a random
/// perfect matching of all doors.
///
/// # Arguments
/// * `n_rooms` - The number of rooms in the map.
/// * `seed` - An optional seed for the random number generator for reproducibility.
///
/// # Returns
/// An `api::Map` struct representing the generated map.
pub fn generate_as_api_map(n_rooms: usize, seed: Option<u64>) -> api::Map {
    let mut rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_os_rng(),
    };

    // Create a list of all doors, identified by a (room, door) tuple.
    let mut all_doors = (0..n_rooms)
        .flat_map(|r| (0..6).map(move |d| (r, d)))
        .collect::<Vec<_>>();
    all_doors.shuffle(&mut rng);

    // Create connections by pairing up the shuffled doors.
    let mut connections = Vec::with_capacity(n_rooms * 6);
    for chunk in all_doors.chunks_exact(2) {
        let (r1, d1) = chunk[0];
        let (r2, d2) = chunk[1];
        // For the API format, we need to represent the undirected edge as two directed edges.
        connections.push(api::MapConnection {
            from: api::MapConnectionEnd { room: r1, door: d1 },
            to: api::MapConnectionEnd { room: r2, door: d2 },
        });
        connections.push(api::MapConnection {
            from: api::MapConnectionEnd { room: r2, door: d2 },
            to: api::MapConnectionEnd { room: r1, door: d1 },
        });
    }

    // The spec defines room signatures as the number of passages, which is always 6
    // in this generator. However, the problem seems to use a 2-bit integer (0-3) as
    // the observable "signature" or "label". This generator assigns it based on index.
    let rooms = (0..n_rooms).map(|i| i % 4).collect::<Vec<_>>();

    api::Map {
        rooms,
        starting_room: rng.random_range(0..n_rooms),
        connections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_as_vec() {
        let n_rooms = 10;
        let map = generate_as_vec(n_rooms, Some(123));

        // Check that the map has the correct number of rooms
        assert_eq!(map.len(), n_rooms);

        // Check that each room has 6 doors
        for (doors, _) in &map {
            assert_eq!(doors.len(), 6);
        }

        // Check that all connections are bidirectional
        for (i, (doors, _)) in map.iter().enumerate() {
            for &neighbor in doors {
                assert!(map[neighbor].0.contains(&i));
            }
        }
    }

    #[test]
    fn test_generate_as_api_map() {
        let n_rooms = 20;
        let api_map = generate_as_api_map(n_rooms, Some(123));

        assert_eq!(api_map.rooms.len(), n_rooms, "rooms: {:?}", api_map.rooms);
        assert_eq!(
            api_map.connections.len(),
            n_rooms * 6,
            "connections: {:?}",
            api_map.connections
        );

        let mut connections_map = std::collections::HashMap::new();
        for conn in &api_map.connections {
            connections_map.insert(
                (conn.from.room, conn.from.door),
                (conn.to.room, conn.to.door),
            );
        }

        assert_eq!(connections_map.len(), n_rooms * 6);

        for r in 0..n_rooms {
            for d in 0..6 {
                assert!(connections_map.contains_key(&(r, d)));
            }
        }
    }
}
