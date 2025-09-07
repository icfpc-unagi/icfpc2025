//! # SVG Map Visualization
//!
//! This module provides functionality to generate an SVG visualization of an
//! Aedificium map structure (`api::Map`). It uses a simple physics-based
//! force-directed layout engine to position the rooms (nodes) in a visually
//! appealing way.

use crate::api;
use rand::Rng;
use svg::Document;
use svg::node::element::path::Data;
use svg::node::element::{Path, Text};

/// Represents a node (a room) in the force-directed layout simulation.
#[derive(Debug, Clone)]
struct Node {
    /// The (x, y) coordinates of the node.
    position: (f64, f64),
    /// The current velocity of the node.
    velocity: (f64, f64),
    /// The net force acting on the node.
    force: (f64, f64),
}

/// A simple force-directed layout engine for positioning graph nodes.
///
/// It simulates physical forces:
/// - A repulsive force between all pairs of nodes (like charged particles).
/// - An attractive force between connected nodes (like springs).
struct LayoutEngine {
    /// The nodes (rooms) in the graph.
    nodes: Vec<Node>,
    /// An adjacency matrix representing the connections (passages).
    adjacency_matrix: Vec<Vec<bool>>,
    /// The strength of the repulsive force.
    k_repel: f64,
    /// The strength of the attractive (spring) force.
    k_attract: f64,
    /// A damping factor to prevent oscillations and help the system stabilize.
    damping: f64,
    /// The time step for the simulation.
    dt: f64,
}

impl LayoutEngine {
    /// Creates a new `LayoutEngine` with randomly initialized node positions.
    fn new(n_nodes: usize, adjacency_matrix: Vec<Vec<bool>>) -> Self {
        let mut nodes = Vec::with_capacity(n_nodes);
        let mut rng = rand::rng();

        for i in 0..n_nodes {
            nodes.push(Node {
                // Initial positions in a grid with slight randomness to break symmetry.
                position: (
                    (i % 10) as f64 * 50.0 + rng.random_range(-5.0..5.0),
                    (i / 10) as f64 * 50.0 + rng.random_range(-5.0..5.0),
                ),
                velocity: (0.0, 0.0),
                force: (0.0, 0.0),
            });
        }
        Self {
            nodes,
            adjacency_matrix,
            k_repel: 100.0,
            k_attract: 0.1,
            damping: 0.9,
            dt: 0.1,
        }
    }

    /// Calculates the net force on each node based on repulsion and attraction.
    fn update_forces(&mut self) {
        const EPSILON: f64 = 1e-6;
        for i in 0..self.nodes.len() {
            self.nodes[i].force = (0.0, 0.0);

            // Repulsive forces (Coulomb's Law): pushes all nodes away from each other.
            for j in 0..self.nodes.len() {
                if i == j {
                    continue;
                }
                let (xi, yi) = self.nodes[i].position;
                let (xj, yj) = self.nodes[j].position;
                let dx = xi - xj;
                let dy = yi - yj;
                let dist_sq = dx * dx + dy * dy;
                let dist = dist_sq.sqrt();

                if dist < EPSILON {
                    // Nodes are too close, apply a strong, constant repulsive force to separate them.
                    let force_magnitude = self.k_repel * 1000.0;
                    self.nodes[i].force.0 += force_magnitude * dx.signum();
                    self.nodes[i].force.1 += force_magnitude * dy.signum();
                    continue;
                }

                let force_magnitude = self.k_repel / dist_sq;
                self.nodes[i].force.0 += force_magnitude * dx / dist;
                self.nodes[i].force.1 += force_magnitude * dy / dist;
            }

            // Attractive forces (Hooke's Law): pulls connected nodes together.
            for j in 0..self.nodes.len() {
                if self.adjacency_matrix[i][j] {
                    let (xi, yi) = self.nodes[i].position;
                    let (xj, yj) = self.nodes[j].position;
                    let dx = xi - xj;
                    let dy = yi - yj;
                    let dist = (dx * dx + dy * dy).sqrt();

                    if dist < EPSILON {
                        continue;
                    }

                    let force_magnitude = self.k_attract * dist;
                    self.nodes[i].force.0 -= force_magnitude * dx / dist;
                    self.nodes[i].force.1 -= force_magnitude * dy / dist;
                }
            }
        }
    }

    /// Updates node velocities and positions based on the calculated forces.
    fn update_positions(&mut self, t: f64) {
        // Use Verlet integration to update positions.
        for i in 0..self.nodes.len() {
            let (vx, vy) = self.nodes[i].velocity;
            let (fx, fy) = self.nodes[i].force;
            let (px, py) = self.nodes[i].position;
            // Apply force and damping to velocity.
            let new_vx = (vx + fx * self.dt) * self.damping;
            let new_vy = (vy + fy * self.dt) * self.damping;
            // Update position based on new velocity.
            let new_px = px + new_vx * self.dt * t;
            let new_py = py + new_vy * self.dt * t;
            self.nodes[i].velocity = (new_vx, new_vy);
            self.nodes[i].position = (new_px, new_py);
        }
    }

    /// Runs the physics simulation for a fixed number of iterations.
    fn run(&mut self, iterations: usize) {
        const EPSILON: f64 = 1e-3;
        for i in 0..iterations {
            // The `t` factor here seems to be a cooling schedule, reducing movement over time.
            let t = ((iterations - i) as f64 / (iterations as f64)).powf(2.0) * 100.0;
            self.update_forces();
            self.update_positions(t);

            // Check if the system has stabilized (i.e., minimal movement).
            let mut stable = true;
            for node in &self.nodes {
                if node.velocity.0.abs() > EPSILON
                    || node.velocity.1.abs() > EPSILON
                    || node.force.0.abs() > EPSILON
                    || node.force.1.abs() > EPSILON
                {
                    stable = false;
                    break;
                }
            }
            if stable {
                // Stop early if the layout is stable.
                break;
            }
        }
    }
}

/// Renders a given `api::Map` into an SVG string.
///
/// The process involves:
/// 1. Creating a `LayoutEngine` to calculate node positions.
/// 2. Running the simulation to stabilize the layout.
/// 3. Normalizing and scaling the final positions to fit in a viewbox.
/// 4. Drawing the passages (connections) as cubic Bezier curves.
/// 5. Drawing the rooms as colored circles with text labels.
pub fn render(map: &api::Map) -> String {
    let n_rooms = map.rooms.len();
    let radius: f64 = 15.0 + 5.0 * (100.0 / n_rooms as f64).sqrt();

    // Set up and run the layout engine.
    let mut adjacency_matrix = vec![vec![false; n_rooms]; n_rooms];
    for conn in &map.connections {
        adjacency_matrix[conn.from.room][conn.to.room] = true;
        adjacency_matrix[conn.to.room][conn.from.room] = true; // Ensure symmetry
    }
    let mut layout_engine = LayoutEngine::new(n_rooms, adjacency_matrix);
    layout_engine.run(1000);
    let mut positions = layout_engine
        .nodes
        .iter()
        .map(|node| node.position)
        .collect::<Vec<_>>();

    // Normalize positions to fit within a standard SVG viewbox.
    let (min_x, min_y, max_x, max_y) = positions
        .iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |acc, &(x, y)| {
            (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y))
        });

    let mut width = (max_x - min_x) * 1.2 + 2.0 * radius;
    let mut height = (max_y - min_y) * 1.2 + 2.0 * radius;

    // Ensure a minimum canvas size.
    width = width.max(500.0);
    height = height.max(500.0);

    let scale_x = if (max_x - min_x).abs() > 1e-6 {
        (width - 2.0 * radius) / (max_x - min_x)
    } else {
        1.0
    };
    let scale_y = if (max_y - min_y).abs() > 1e-6 {
        (height - 2.0 * radius) / (max_y - min_y)
    } else {
        1.0
    };
    let scale = scale_x.min(scale_y);

    for pos in &mut positions {
        pos.0 = (pos.0 - min_x) * scale + radius;
        pos.1 = (pos.1 - min_y) * scale + radius;
    }

    let mut document = Document::new();

    // Draw connections (passages) as curved paths.
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for conn in &map.connections {
        // Only draw each edge once for an undirected graph.
        if conn.from.room >= conn.to.room {
            continue;
        }
        let p1 = positions[conn.from.room];
        let p2 = positions[conn.to.room];

        let angle1 = (conn.from.door as f64) * std::f64::consts::PI / 3.0;
        let c1 = (p1.0 + radius * angle1.cos(), p1.1 + radius * angle1.sin());

        let angle2 = (conn.to.door as f64) * std::f64::consts::PI / 3.0;
        let c2 = (p2.0 + radius * angle2.cos(), p2.1 + radius * angle2.sin());

        let dist = ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt();

        // Use a cubic Bezier curve for a nice arc.
        let a1x = c1.0 + (c1.0 - p1.0) / radius * dist * 0.4;
        let a1y = c1.1 + (c1.1 - p1.1) / radius * dist * 0.4;
        let a2x = c2.0 + (c2.0 - p2.0) / radius * dist * 0.4;
        let a2y = c2.1 + (c2.1 - p2.1) / radius * dist * 0.4;
        let data = Data::new()
            .move_to((c1.0, c1.1))
            .cubic_curve_to((a1x, a1y, a2x, a2y, c2.0, c2.1));
        min_x = min_x.min(c1.0).min(c2.0).min(a1x).min(a2x);
        min_y = min_y.min(c1.1).min(c2.1).min(a1y).min(a2y);
        max_x = max_x.max(c1.0).max(c2.0).max(a1x).max(a2x);
        max_y = max_y.max(c1.1).max(c2.1).max(a1y).max(a2y);

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 2)
            .set("d", data)
            .set("title", format!("{} <-> {}", conn.from.room, conn.to.room))
            .set("onmouseover", "this.setAttribute('stroke-width', 4)")
            .set("onmouseout", "this.setAttribute('stroke-width', 2)");

        document = document.add(path);
    }

    // Draw rooms as circles.
    for (i, pos) in positions.iter().enumerate() {
        let color = match map.rooms[i] {
            0 => "#1f77b4",
            1 => "#ff7f0e",
            2 => "#2ca02c",
            _ => "#d62728",
        };

        let circle = svg::node::element::Circle::new()
            .set("cx", pos.0)
            .set("cy", pos.1)
            .set("r", radius)
            .set("fill", color)
            .set("stroke", "black")
            .set("stroke-width", 2)
            .set("title", format!("Room {}, Signature {}", i, map.rooms[i]));
        document = document.add(circle);

        // Add text label inside the circle.
        let text = Text::new(format!("{}#{}", i, map.rooms[i]))
            .set("x", pos.0)
            .set("y", pos.1 + 7.0)
            .set("text-anchor", "middle")
            .set("font-size", "20px");
        document = document.add(text);
    }
    document = document
        .set("width", max_x - min_x)
        .set("height", max_y - min_y)
        .set("viewBox", (min_x, min_y, max_x - min_x, max_y - min_y));

    document.to_string()
}

#[cfg(test)]
mod tests {
    use crate::{api, svg};

    #[test]
    fn test_svg_render_small_map() {
        let map = api::Map {
            rooms: vec![0, 1],
            starting_room: 0,
            connections: vec![api::MapConnection {
                from: api::MapConnectionEnd { room: 0, door: 0 },
                to: api::MapConnectionEnd { room: 1, door: 1 },
            }],
        };
        let svg_str = svg::render(&map);
        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("<circle"));
        assert!(svg_str.contains("<path"));
        assert!(svg_str.contains("Room 0, Signature 0"));
        assert!(svg_str.contains("Room 1, Signature 1"));
    }

    #[test]
    fn test_svg_render_single_room() {
        let map = api::Map {
            rooms: vec![2],
            starting_room: 0,
            connections: vec![],
        };
        let svg_str = svg::render(&map);
        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("<circle"));
        assert!(svg_str.contains("Room 0, Signature 2"));
    }
}
