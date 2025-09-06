use crate::api;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

/// Generates a random map of n rooms: room index -> (door -> connected room index, room hash)
pub fn generate_as_vec(n_rooms: usize, seed: Option<u64>) -> Vec<([usize; 6], u8)> {
    let mut rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_os_rng(),
    };

    // List all 6 doors (unnumberred) in all rooms, and connect in random pairs.
    let mut doors: Vec<usize> = (0..n_rooms)
        .flat_map(|i| std::iter::repeat_n(i, 6))
        .collect();
    doors.shuffle(&mut rng);
    let conns: Vec<_> = doors
        .chunks_exact(2)
        .map(|chunk| (chunk[0], chunk[1]))
        .collect();

    // Build adjacency list.
    let mut adj = vec![vec![]; n_rooms];
    for &(a, b) in &conns {
        adj[a].push(b);
        adj[b].push(a);
    }

    // Construct the output with room hash.
    let mut map: Vec<([usize; 6], u8)> = Vec::with_capacity(n_rooms);
    for (i, neighbors) in adj.iter().enumerate() {
        let hash = (i % 4) as u8;
        let mut doors = [0; 6];
        doors.copy_from_slice(&neighbors[..6]);
        map.push((doors, hash));
    }
    map
}

pub fn generate_as_api_map(n_rooms: usize, seed: Option<u64>) -> api::Map {
    let mut rng = match seed {
        Some(s) => rand::rngs::StdRng::seed_from_u64(s),
        None => rand::rngs::StdRng::from_os_rng(),
    };

    // Create a list of all doors for all rooms.
    let mut all_doors = (0..n_rooms)
        .flat_map(|r| (0..6).map(move |d| (r, d)))
        .collect::<Vec<_>>();
    all_doors.shuffle(&mut rng);

    // Create connections by pairing up the doors.
    let mut connections = Vec::with_capacity(n_rooms * 6);
    for chunk in all_doors.chunks_exact(2) {
        let (r1, d1) = chunk[0];
        let (r2, d2) = chunk[1];
        connections.push(api::MapConnection {
            from: api::MapConnectionEnd { room: r1, door: d1 },
            to: api::MapConnectionEnd { room: r2, door: d2 },
        });
        connections.push(api::MapConnection {
            from: api::MapConnectionEnd { room: r2, door: d2 },
            to: api::MapConnectionEnd { room: r1, door: d1 },
        });
    }

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
