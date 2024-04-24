#![warn(missing_docs)]
#![allow(clippy::tabs_in_doc_comments)]

//! Implementation of [ForceAtlas2](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4051631/) &#8211; force-directed Continuous Graph Layout Algorithm for Handy Network Visualization (i.e. position the nodes of a n-dimension graph for drawing it more human-readably)
//!
//! This example creates a graph containing 4 nodes of size 1, linked by undirected weighted edges.
//! ```rust
//! let mut layout = forceatlas2::Layout::<f32, 2>::from_graph_with_degree_mass(
//! 	vec![((0, 1), 1.0), ((1, 2), 1.5), ((0, 2), 0.7), ((2, 3), 1.0)],
//! 	(0..4).map(|_| 1.0),
//! 	forceatlas2::Settings::default(),
//! );
//! for _ in 0..100 {
//! 	layout.iteration();
//! }
//! for (i, node) in layout.nodes.iter().enumerate() {
//! 	println!("{}: ({}, {})", i, node.pos.x(), node.pos.y());
//! }
//! ```

mod forces;
mod layout;
mod trees;
mod util;

use forces::{Attraction, Gravity, Repulsion};

pub use layout::{Layout, Node, Settings};
pub use util::{Coord, Edge, Vec2, Vec3, VecN};

use num_traits::Zero;

impl<T: Coord, const N: usize> Layout<T, N>
where
	Layout<T, N>: forces::Attraction<T, N> + forces::Gravity<T, N> + forces::Repulsion<T, N>,
{
	/// Create an empty layout
	pub fn empty(settings: Settings<T>) -> Self {
		assert!(settings.check());
		Self {
			edges: Vec::new(),
			nodes: Vec::new(),
			bump: parking_lot::Mutex::new(bumpalo::Bump::new()),
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: Self::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Create a randomly positioned layout from an undirected graph, using degree as mass
	///
	/// As many nodes will be created as elements in `sizes`.
	#[cfg(feature = "rand")]
	pub fn from_graph_with_degree_mass<I: IntoIterator<Item = T>>(
		mut edges: Vec<(Edge, T)>,
		sizes: I,
		settings: Settings<T>,
	) -> Self
	where
		rand::distributions::Standard: rand::distributions::Distribution<T>,
		T: rand::distributions::uniform::SampleUniform,
	{
		assert!(settings.check());

		let mut nodes: Vec<Node<T, N>> = {
			let mut rng = rand::thread_rng();
			sizes
				.into_iter()
				.map(|size| Node {
					pos: util::sample_unit_cube(&mut rng),
					mass: T::zero(),
					speed: Zero::zero(),
					old_speed: Zero::zero(),
					size,
				})
				.collect()
		};

		for (edge, _weight) in edges.iter_mut() {
			nodes
				.get_mut(edge.0)
				.expect("Node index out of bound in edge list")
				.mass += T::one();
			nodes
				.get_mut(edge.1)
				.expect("Node index out of bound in edge list")
				.mass += T::one();
			if edge.0 > edge.1 {
				*edge = (edge.1, edge.0);
			}
		}

		let nb = nodes.len() * N;
		Self {
			edges,
			nodes,
			bump: parking_lot::Mutex::new(bumpalo::Bump::with_capacity(
				(nb + 4 * (nb.checked_ilog2().unwrap_or(0) as usize + 1))
					* std::mem::size_of::<
						trees::NodeN<T, forces::repulsion::NodeBodyN<T, T, N>, N, 1>,
					>(),
			)),
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: Self::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Create a randomly positioned layout from an undirected graph
	///
	/// `masses_sizes`'s elements are `(mass, size)`.
	///
	/// As many nodes will be created as elements in `masses_sizes`.
	#[cfg(feature = "rand")]
	pub fn from_graph_with_masses<I: IntoIterator<Item = (T, T)>>(
		mut edges: Vec<(Edge, T)>,
		masses_sizes: I,
		settings: Settings<T>,
	) -> Self
	where
		rand::distributions::Standard: rand::distributions::Distribution<T>,
		T: rand::distributions::uniform::SampleUniform,
	{
		assert!(settings.check());

		let nodes: Vec<Node<T, N>> = {
			let mut rng = rand::thread_rng();
			masses_sizes
				.into_iter()
				.map(|(mass, size)| Node {
					pos: util::sample_unit_cube(&mut rng),
					mass,
					speed: Zero::zero(),
					old_speed: Zero::zero(),
					size,
				})
				.collect()
		};
		for (edge, _weight) in edges.iter_mut() {
			assert!(
				edge.0 < nodes.len() && edge.1 < nodes.len(),
				"Node index out of bound in edge list"
			);
			if edge.0 > edge.1 {
				*edge = (edge.1, edge.0);
			}
		}

		let nb = nodes.len() * N;
		Self {
			edges,
			nodes,
			bump: parking_lot::Mutex::new(bumpalo::Bump::with_capacity(
				(nb + 4 * (nb.checked_ilog2().unwrap_or(0) as usize + 1))
					* std::mem::size_of::<
						trees::NodeN<T, forces::repulsion::NodeBodyN<T, T, N>, N, 1>,
					>(),
			)),
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: Self::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Create a layout from an undirected graph, with initial positions
	pub fn from_position_graph(
		mut edges: Vec<(Edge, T)>,
		nodes: Vec<Node<T, N>>,
		settings: Settings<T>,
	) -> Self {
		assert!(settings.check());

		for (edge, _weight) in edges.iter_mut() {
			if edge.0 > edge.1 {
				*edge = (edge.1, edge.0);
			}
		}

		Self {
			bump: parking_lot::Mutex::new(bumpalo::Bump::with_capacity(
				(nodes.len() + 4 * (nodes.len().checked_ilog2().unwrap_or(0) as usize + 1))
					* std::mem::size_of::<
						trees::NodeN<T, forces::repulsion::NodeBodyN<T, T, N>, N, 1>,
					>(),
			)),
			edges,
			nodes,
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: Self::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Get layout's settings
	pub fn get_settings(&self) -> &Settings<T> {
		&self.settings
	}

	/// Add nodes to the graph
	///
	/// New node indices in edges start at the current number of nodes.
	pub fn add_nodes(&mut self, edges: &[(Edge, T)], nodes: &[Node<T, N>]) {
		self.nodes.extend_from_slice(nodes);
		self.edges.extend(
			edges
				.iter()
				.map(|((n1, n2), weight)| (if n1 > n2 { (*n2, *n1) } else { (*n1, *n2) }, *weight)),
		);
	}

	/// Remove an edge by index
	pub fn remove_edge(&mut self, edge: usize) {
		self.edges.remove(edge);
	}

	/// Remove a node by index
	///
	/// Assumes it has a null degree (if not, next iteration will panic)
	pub fn remove_node(&mut self, node: usize) {
		self.nodes.remove(node);
	}

	/// Remove a node's incident edges
	pub fn remove_incident_edges(&mut self, node: usize) {
		util::drain_filter_swap(&mut self.edges, |((n1, n2), _weight)| {
			if *n1 == node || *n2 == node {
				false
			} else {
				if *n1 > node {
					*n1 -= 1;
				}
				if *n2 > node {
					*n2 -= 1;
				}
				true
			}
		});
	}

	/// Remove a node by index, automatically removing all its incident edges
	pub fn remove_node_with_edges(&mut self, node: usize) {
		self.remove_incident_edges(node);
		self.remove_node(node);
	}

	/// Change layout settings
	pub fn set_settings(&mut self, settings: Settings<T>) {
		assert!(settings.check());
		self.fn_attraction = Self::choose_attraction(&settings);
		self.fn_gravity = Self::choose_gravity(&settings);
		self.fn_repulsion = Self::choose_repulsion(&settings);
		self.settings = settings;
	}

	/// Compute an iteration of ForceAtlas2
	pub fn iteration(&mut self) {
		self.init_iteration();
		self.apply_attraction();
		self.apply_repulsion();
		self.apply_gravity();
		self.apply_forces();
	}

	fn init_iteration(&mut self) {
		for node in self.nodes.iter_mut() {
			node.old_speed = node.speed;
			node.speed = Zero::zero();
		}
	}

	fn apply_attraction(&mut self) {
		(self.fn_attraction)(self)
	}

	fn apply_gravity(&mut self) {
		(self.fn_gravity)(self)
	}

	fn apply_repulsion(&mut self) {
		(self.fn_repulsion)(self)
	}

	fn apply_forces(&mut self) {
		for node in self.nodes.iter_mut() {
			let swinging = node
				.speed
				.iter()
				.zip(node.old_speed.iter())
				.map(|(s, old_s)| (*s - *old_s) * (*s - *old_s))
				.sum::<T>()
				.sqrt();
			let traction = node
				.speed
				.iter()
				.zip(node.old_speed.iter())
				.map(|(s, old_s)| (*s + *old_s) * (*s + *old_s))
				.sum::<T>()
				.sqrt();

			let f = traction.ln_1p() / (swinging.sqrt() + T::one()) * self.settings.speed;

			node.pos
				.iter_mut()
				.zip(node.speed.iter_mut())
				.for_each(|(pos, speed)| {
					*pos += *speed * f;
				});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[cfg(feature = "rand")]
	#[test]
	fn test_global() {
		let mut layout = Layout::<f64, 2>::from_graph_with_degree_mass(
			vec![
				((0, 1), 1.0),
				((0, 2), 1.0),
				((0, 3), 1.0),
				((1, 2), 1.0),
				((1, 4), 1.0),
			],
			[1.0; 5],
			Settings::default(),
		);

		for _ in 0..10 {
			layout.iteration();
		}

		layout
			.nodes
			.iter()
			.for_each(|node| println!("{:?}", node.pos));
	}

	#[test]
	fn test_init_iteration() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![((0, 1), 1.0)],
			vec![
				Node {
					pos: VecN([-1.0, -1.0]),
					speed: VecN([12.34, 56.78]),
					..Default::default()
				},
				Node {
					pos: VecN([1.0, 1.0]),
					speed: VecN([42.0, 666.0]),
					..Default::default()
				},
			],
			Settings::default(),
		);
		layout.init_iteration();
		assert_eq!(layout.nodes[0].speed, VecN([0.0, 0.0]));
		assert_eq!(layout.nodes[1].speed, VecN([0.0, 0.0]));
	}

	#[test]
	fn test_forces() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![((0, 1), 1.0)],
			vec![
				Node {
					pos: VecN([-2.0, -2.0]),
					..Default::default()
				},
				Node {
					pos: VecN([1.0, 2.0]),
					..Default::default()
				},
			],
			Settings::default(),
		);

		layout.init_iteration();
		layout.apply_attraction();

		let speed_1 = dbg!(layout.nodes[0].speed);
		let speed_2 = dbg!(layout.nodes[1].speed);

		assert!(speed_1[0] > 0.0);
		assert!(speed_1[1] > 0.0);
		assert!(speed_2[0] < 0.0);
		assert!(speed_2[1] < 0.0);
		assert_eq!(speed_1[0], 3.0);
		assert_eq!(speed_1[1], 4.0);
		assert_eq!(speed_2[0], -3.0);
		assert_eq!(speed_2[1], -4.0);

		layout.init_iteration();
		layout.apply_repulsion();

		let speed_1 = dbg!(layout.nodes[0].speed);
		let speed_2 = dbg!(layout.nodes[1].speed);

		assert!(speed_1[0] < 0.0);
		assert!(speed_1[1] < 0.0);
		assert!(speed_2[0] > 0.0);
		assert!(speed_2[1] > 0.0);
		assert!(speed_1[0] > -10.0);
		assert!(speed_1[1] > -10.0);
		assert!(speed_2[0] < 10.0);
		assert!(speed_2[1] < 10.0);

		layout.init_iteration();
		layout.apply_gravity();

		let speed_1 = dbg!(layout.nodes[0].speed);
		let speed_2 = dbg!(layout.nodes[1].speed);

		assert!(speed_1[0] > 0.0);
		assert!(speed_1[1] > 0.0);
		assert!(speed_2[0] < 0.0);
		assert!(speed_2[1] < 0.0);
	}

	#[test]
	fn test_convergence() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![((0, 1), 1.0), ((1, 2), 1.0)],
			vec![
				Node {
					pos: VecN([-1.1, -1.0]),
					..Default::default()
				},
				Node {
					pos: VecN([0.0, 0.0]),
					..Default::default()
				},
				Node {
					pos: VecN([1.0, 1.0]),
					..Default::default()
				},
			],
			Settings {
				ka: 0.5,
				kg: 0.01,
				kr: 0.01,
				lin_log: false,
				prevent_overlapping: None,
				speed: 1.0,
				strong_gravity: false,
				theta: 0.5,
			},
		);

		for _ in 0..10 {
			println!("new iteration");
			layout.init_iteration();
			layout.apply_attraction();
			layout.init_iteration();
			layout.apply_repulsion();
			layout.init_iteration();
			layout.apply_gravity();
			layout.apply_forces();

			let point_1 = layout.nodes[0].pos;
			let point_2 = layout.nodes[1].pos;
			dbg!(((point_2[0] - point_1[0]).powi(2) + (point_2[1] - point_1[1]).powi(2)).sqrt());
		}
	}

	#[test]
	fn test_convergence_po() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![((0, 1), 1.0), ((1, 2), 1.0)],
			vec![
				Node {
					pos: VecN([-1.1, -1.0]),
					size: 1.0,
					..Default::default()
				},
				Node {
					pos: VecN([0.0, 0.0]),
					size: 5.0,
					..Default::default()
				},
				Node {
					pos: VecN([1.0, 1.0]),
					size: 1.0,
					..Default::default()
				},
			],
			Settings {
				ka: 0.5,
				kg: 0.01,
				kr: 0.01,
				lin_log: false,
				prevent_overlapping: Some(100.),
				speed: 1.0,
				strong_gravity: false,
				theta: 0.5,
			},
		);

		for _ in 0..10 {
			println!("new iteration");
			layout.init_iteration();
			layout.apply_attraction();
			layout.init_iteration();
			layout.apply_repulsion();
			layout.init_iteration();
			layout.apply_gravity();
			layout.apply_forces();

			let point_1 = layout.nodes[0].pos;
			let point_2 = layout.nodes[1].pos;
			dbg!(((point_2[0] - point_1[0]).powi(2) + (point_2[1] - point_1[1]).powi(2)).sqrt());
		}
	}
}
