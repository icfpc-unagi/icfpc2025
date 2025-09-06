use crate::api;
use rand::Rng;
use svg::Document;
use svg::node::element::path::Data;
use svg::node::element::{Path, Text};

#[derive(Debug, Clone)]
struct Node {
    position: (f64, f64),
    velocity: (f64, f64),
    force: (f64, f64),
}

struct LayoutEngine {
    nodes: Vec<Node>,
    adjacency_matrix: Vec<Vec<bool>>,
    k_repel: f64,
    k_attract: f64,
    damping: f64,
    dt: f64,
}

impl LayoutEngine {
    fn new(n_nodes: usize, adjacency_matrix: Vec<Vec<bool>>) -> Self {
        let mut nodes = Vec::with_capacity(n_nodes);
        let mut rng = rand::rng();

        for i in 0..n_nodes {
            nodes.push(Node {
                // Initial positions in a grid with slight randomness
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

    fn update_forces(&mut self) {
        const EPSILON: f64 = 1e-6;
        for i in 0..self.nodes.len() {
            self.nodes[i].force = (0.0, 0.0);

            // Repulsive forces
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
                    // Nodes are too close, apply a strong repulsive force
                    let force_magnitude = self.k_repel * 1000.0; // Large constant force
                    self.nodes[i].force.0 += force_magnitude * dx.signum();
                    self.nodes[i].force.1 += force_magnitude * dy.signum();
                    continue;
                }

                let force_magnitude = self.k_repel / dist_sq;
                self.nodes[i].force.0 += force_magnitude * dx / dist;
                self.nodes[i].force.1 += force_magnitude * dy / dist;
            }

            // Attractive forces
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

    fn update_positions(&mut self, t: f64) {
        // Update velocities and positions
        for i in 0..self.nodes.len() {
            let (vx, vy) = self.nodes[i].velocity;
            let (fx, fy) = self.nodes[i].force;
            let (px, py) = self.nodes[i].position;
            let new_vx = (vx + fx * self.dt) * self.damping;
            let new_vy = (vy + fy * self.dt) * self.damping;
            let new_px = px + new_vx * self.dt * t;
            let new_py = py + new_vy * self.dt * t;
            self.nodes[i].velocity = (new_vx, new_vy);
            self.nodes[i].position = (new_px, new_py);
        }
    }

    fn run(&mut self, iterations: usize) {
        const EPSILON: f64 = 1e-3;
        for i in 0..iterations {
            let t = ((iterations - i) as f64 / (iterations as f64)).powf(2.0) * 100.0;
            self.update_forces();
            self.update_positions(t);
            // eprintln!("Iteration {}/{}", i + 1, iterations);
            let mut stable = true;
            for node in &self.nodes {
                if node.velocity.0.abs() > EPSILON
                    || node.velocity.1.abs() > EPSILON
                    || node.force.0.abs() > EPSILON
                    || node.force.1.abs() > EPSILON
                {
                    stable = false;
                }
                // eprintln!(
                //     "p=({:.2},{:.2}) v=({:.2},{:.2}) f=({:.2},{:.2})",
                //     node.position.0,
                //     node.position.1,
                //     node.velocity.0,
                //     node.velocity.1,
                //     node.force.0,
                //     node.force.1,
                // );
            }
            if stable {
                break;
            }
        }
    }
}

pub fn render(map: &api::Map) -> String {
    let n_rooms = map.rooms.len();
    let radius: f64 = 15.0 + 5.0 * (100.0 / n_rooms as f64).sqrt();

    let mut adjacency_matrix = vec![vec![false; n_rooms]; n_rooms];
    for conn in &map.connections {
        adjacency_matrix[conn.from.room][conn.to.room] = true;
    }
    let mut layout_engine = LayoutEngine::new(n_rooms, adjacency_matrix);
    layout_engine.run(1000);
    let mut positions = layout_engine
        .nodes
        .iter()
        .map(|node| node.position)
        .collect::<Vec<_>>();
    let _forces = layout_engine
        .nodes
        .iter()
        .map(|node| node.force)
        .collect::<Vec<_>>();

    // Normalize positions to fit within the SVG viewbox
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for &(x, y) in &positions {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    let mut width = (max_x - min_x) * 1.2 + 2.0 * radius;
    let mut height = (max_y - min_y) * 1.2 + 2.0 * radius;

    // Ensure minimum size
    if width < 500.0 {
        width = 500.0;
    }
    if height < 500.0 {
        height = 500.0;
    }

    let scale_x = if (max_x - min_x) > 0.0 {
        (width - 2.0 * radius) / (max_x - min_x)
    } else {
        1.0
    };
    let scale_y = if (max_y - min_y) > 0.0 {
        (height - 2.0 * radius) / (max_y - min_y)
    } else {
        1.0
    };
    let scale = scale_x.min(scale_y);

    for pos in &mut positions {
        pos.0 = (pos.0 - min_x) * scale + radius;
        pos.1 = (pos.1 - min_y) * scale + radius;
    }

    let mut document = Document::new()
        .set("width", width + 20.0)
        .set("height", height + 20.0)
        .set("viewBox", (-10.0, -10.0, width + 10.0, height + 10.0));

    // Draw connections
    for conn in &map.connections {
        let p1 = positions[conn.from.room];
        let p2 = positions[conn.to.room];

        let angle1 = (conn.from.door as f64) * std::f64::consts::PI / 3.0;
        let c1 = (p1.0 + radius * angle1.cos(), p1.1 + radius * angle1.sin());

        let angle2 = (conn.to.door as f64) * std::f64::consts::PI / 3.0;
        let c2 = (p2.0 + radius * angle2.cos(), p2.1 + radius * angle2.sin());

        let dist = ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt();

        let data = Data::new().move_to((c1.0, c1.1)).cubic_curve_to((
            c1.0 + (c1.0 - p1.0) / radius * dist * 0.4,
            c1.1 + (c1.1 - p1.1) / radius * dist * 0.4,
            c2.0 + (c2.0 - p2.0) / radius * dist * 0.4,
            c2.1 + (c2.1 - p2.1) / radius * dist * 0.4,
            c2.0,
            c2.1,
        ));

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

    // Draw rooms
    for (i, pos) in positions.iter().enumerate() {
        let color = match map.rooms[i] {
            0 => "#ff8080",
            1 => "#80ff80",
            2 => "#8080ff",
            _ => "#ffff80",
        };

        let circle = svg::node::element::Circle::new()
            .set("cx", pos.0)
            .set("cy", pos.1)
            .set("r", radius)
            .set("fill", color)
            .set("stroke", "black")
            .set("stroke-width", 2)
            .set("title", format!("Index {}, Hash {}", i, map.rooms[i]));
        document = document.add(circle);

        let text = Text::new(format!("{}#{}", i, map.rooms[i]))
            .set("x", pos.0)
            .set("y", pos.1 + 7.0)
            .set("text-anchor", "middle")
            .set("font-size", "20px");
        document = document.add(text);

        // let force = forces[i];
        // let arrow: Path = Path::new()
        //     .set("fill", "none")
        //     .set("stroke", "#ff0000")
        //     .set("stroke-width", 4)
        //     .set(
        //         "d",
        //         Data::new()
        //             .move_to((pos.0, pos.1))
        //             .line_to((pos.0 + force.0 * 0.1, pos.1 + force.1 * 0.1)),
        //     )
        //     .set("marker-end", "url(#arrowhead)");
        // document = document.add(arrow);
    }

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
        // assert!(svg_str.contains("0:0"));
        // assert!(svg_str.contains("1:1"));
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
        // assert!(svg_str.contains("0:2"));
    }
}
