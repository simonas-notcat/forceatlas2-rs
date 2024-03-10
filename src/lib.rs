#![feature(trait_alias)]

mod forces;
mod layout;
mod trees;
mod util;

use forces::{Attraction, Repulsion};

pub use layout::{Layout, Settings};
pub use util::{Coord, Edge, Nodes};

use num_traits::cast::NumCast;

impl<T: Coord, const N: usize> Layout<T, N>
where
	Layout<T, N>: forces::Repulsion<T, N> + forces::Attraction<T, N>,
{
	/// Instantiates an empty layout
	pub fn empty(weighted: bool, settings: Settings<T>) -> Self {
		assert!(settings.check());
		Self {
			edges: Vec::new(),
			points: Vec::new(),
			masses: Vec::new(),
			sizes: if settings.prevent_overlapping.is_some() {
				Some(Vec::new())
			} else {
				None
			},
			speeds: Vec::new(),
			old_speeds: Vec::new(),
			weights: if weighted { Some(Vec::new()) } else { None },
			bump: parking_lot::Mutex::new(bumpalo::Bump::new()),
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: forces::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Instanciates a randomly positioned layout from an undirected graph
	///
	/// Assumes edges `(n1, n2)` respect `n1 < n2`.
	#[cfg(feature = "rand")]
	pub fn from_graph(
		edges: Vec<Edge>,
		nodes: Nodes<T>,
		sizes: Option<Vec<T>>,
		weights: Option<Vec<T>>,
		settings: Settings<T>,
	) -> Self
	where
		rand::distributions::Standard: rand::distributions::Distribution<T>,
		T: rand::distributions::uniform::SampleUniform,
	{
		assert!(settings.check());
		if let Some(weights) = &weights {
			assert_eq!(weights.len(), edges.len());
		}

		let nodes = match nodes {
			Nodes::Degree(nb_nodes) => {
				let mut degrees: Vec<usize> = vec![0; nb_nodes];
				for (n1, n2) in edges.iter() {
					*degrees.get_mut(*n1).unwrap() += 1;
					*degrees.get_mut(*n2).unwrap() += 1;
				}
				degrees
					.into_iter()
					.map(|degree| <T as NumCast>::from(degree).unwrap())
					.collect()
			}
			Nodes::Mass(masses) => masses,
		};

		if let Some(sizes) = &sizes {
			assert_eq!(sizes.len(), nodes.len());
		} else {
			assert!(settings.prevent_overlapping.is_none());
		}

		let nb = nodes.len() * N;
		Self {
			edges,
			points: {
				let mut rng = rand::thread_rng();
				(0..nodes.len())
					.map(|_| util::sample_unit_ncube(&mut rng))
					.collect()
			},
			masses: nodes,
			sizes,
			speeds: (0..nb).map(|_| [T::zero(); N]).collect(),
			old_speeds: (0..nb).map(|_| [T::zero(); N]).collect(),
			weights,
			bump: parking_lot::Mutex::new(bumpalo::Bump::with_capacity(
				(nb + 4 * (nb.checked_ilog2().unwrap_or(0) as usize + 1))
					* std::mem::size_of::<trees::NodeN<T, forces::repulsion::NodeBodyN<T, N>, N, 1>>(
					),
			)),
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: forces::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	/// Instanciates layout from an undirected graph, using initial positions
	///
	/// Assumes edges `(n1, n2)` respect `n1 < n2`
	///
	/// `positions` is a list of coordinates, e.g. `[x1, y1, x2, y2, ...]`.
	pub fn from_position_graph(
		edges: Vec<Edge>,
		nodes: Nodes<T>,
		sizes: Option<Vec<T>>,
		positions: Vec<[T; N]>,
		weights: Option<Vec<T>>,
		settings: Settings<T>,
	) -> Self {
		assert!(settings.check());
		if let Some(weights) = &weights {
			assert_eq!(weights.len(), edges.len());
		}

		let nodes = match nodes {
			Nodes::Degree(nb_nodes) => {
				let mut degrees: Vec<usize> = vec![0; nb_nodes];
				for (n1, n2) in edges.iter() {
					*degrees.get_mut(*n1).unwrap() += 1;
					*degrees.get_mut(*n2).unwrap() += 1;
				}
				degrees
					.into_iter()
					.map(|degree| <T as NumCast>::from(degree).unwrap())
					.collect()
			}
			Nodes::Mass(masses) => masses,
		};

		if let Some(sizes) = &sizes {
			assert_eq!(sizes.len(), nodes.len());
		} else {
			assert!(settings.prevent_overlapping.is_none());
		}

		assert_eq!(positions.len(), nodes.len());
		Self {
			bump: parking_lot::Mutex::new(bumpalo::Bump::with_capacity(
				(nodes.len() + 4 * (nodes.len().checked_ilog2().unwrap_or(0) as usize + 1))
					* std::mem::size_of::<trees::NodeN<T, forces::repulsion::NodeBodyN<T, N>, N, 1>>(
					),
			)),
			edges,
			sizes,
			points: positions,
			speeds: (0..nodes.len()).map(|_| [T::zero(); N]).collect(),
			old_speeds: (0..nodes.len()).map(|_| [T::zero(); N]).collect(),
			masses: nodes,
			weights,
			fn_attraction: Self::choose_attraction(&settings),
			fn_gravity: forces::choose_gravity(&settings),
			fn_repulsion: Self::choose_repulsion(&settings),
			settings,
		}
	}

	pub fn get_settings(&self) -> &Settings<T> {
		&self.settings
	}

	/// New node indices in arguments start at the current number of nodes
	pub fn add_nodes(
		&mut self,
		edges: &[Edge],
		nodes: Nodes<T>,
		positions: &[[T; N]],
		weights: Option<&[T]>,
	) {
		let new_nodes;
		match nodes {
			Nodes::Degree(nb_nodes) => {
				new_nodes = nb_nodes;
				self.masses.extend((0..nb_nodes).map(|_| T::zero()));
				for (n1, n2) in edges.iter() {
					self.masses[*n1] += T::one();
					self.masses[*n2] += T::one();
				}
			}
			Nodes::Mass(masses) => {
				new_nodes = masses.len();
				self.masses.extend_from_slice(&masses);
			}
		}
		assert_eq!(positions.len(), new_nodes);
		self.points.extend_from_slice(positions);
		self.speeds
			.extend((0..positions.len()).map(|_| [T::zero(); N]));
		self.old_speeds
			.extend((0..positions.len()).map(|_| [T::zero(); N]));
		self.edges.extend_from_slice(edges);
		match (weights, &mut self.weights) {
			(Some(new_weights), Some(weights)) => {
				assert_eq!(edges.len(), new_weights.len());
				weights.extend_from_slice(new_weights);
			}
			(None, None) => {}
			_ => panic!("Inconsistent weighting"),
		}
	}

	/// Remove edges by index
	pub fn remove_edge(&mut self, edge: usize) {
		self.edges.remove(edge);
		if let Some(weights) = &mut self.weights {
			weights.remove(edge);
		}
	}

	/// Remove a node by index
	///
	/// Assumes it has a null degree
	pub fn remove_node(&mut self, node: usize) {
		self.points.remove(node);
		self.masses.remove(node);
		self.speeds.remove(node);
		self.old_speeds.remove(node);
	}

	/// Remove a node's incident edges
	pub fn remove_incident_edges(&mut self, node: usize) {
		util::drain_filter_swap(&mut self.edges, |(n1, n2)| {
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

	/// Changes layout settings
	pub fn set_settings(&mut self, settings: Settings<T>) {
		assert!(settings.check());
		self.fn_attraction = Self::choose_attraction(&settings);
		self.fn_gravity = forces::choose_gravity(&settings);
		self.fn_repulsion = Self::choose_repulsion(&settings);
		self.settings = settings;
	}

	/// Computes an iteration of ForceAtlas2
	pub fn iteration(&mut self) {
		self.init_iteration();
		self.apply_attraction();
		self.apply_repulsion();
		self.apply_gravity();
		self.apply_forces();
	}

	fn init_iteration(&mut self) {
		for (speed, old_speed) in self.speeds.iter_mut().zip(self.old_speeds.iter_mut()) {
			*old_speed = *speed;
			*speed = [T::zero(); N];
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
		for ((pos, speed), old_speed) in self
			.points
			.iter_mut()
			.zip(self.speeds.iter_mut())
			.zip(self.old_speeds.iter())
		{
			let swinging = speed
				.iter()
				.zip(old_speed.iter())
				.map(|(s, old_s)| (*s - *old_s) * (*s - *old_s))
				.sum::<T>()
				.sqrt();
			let traction = speed
				.iter()
				.zip(old_speed.iter())
				.map(|(s, old_s)| (*s + *old_s) * (*s + *old_s))
				.sum::<T>()
				.sqrt();

			let f = traction.ln_1p() / (swinging.sqrt() + T::one()) * self.settings.speed;

			pos.iter_mut()
				.zip(speed.iter_mut())
				.for_each(|(pos, speed)| {
					*pos += *speed * f;
				});
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	use alloc_counter::{deny_alloc, AllocCounterSystem};

	#[global_allocator]
	static A: AllocCounterSystem = AllocCounterSystem;

	#[cfg(feature = "rand")]
	#[test]
	fn test_global() {
		let mut layout = Layout::<f64, 2>::from_graph(
			vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 4)],
			Nodes::Degree(5),
			None,
			None,
			Settings::default(),
		);

		for _ in 0..10 {
			layout.iteration();
		}

		layout.points.iter().for_each(|pos| println!("{:?}", pos));
	}

	#[test]
	fn test_init_iteration() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![(0, 1)],
			Nodes::Degree(2),
			None,
			vec![[-1.0, -1.0], [1.0, 1.0]],
			None,
			Settings::default(),
		);
		layout
			.speeds
			.iter_mut()
			.enumerate()
			.for_each(|(i, s)| *s = [i as f64, i as f64]);
		layout.init_iteration();
		assert_eq!(&layout.speeds, &[[0.0, 0.0], [0.0, 0.0]]);
	}

	#[test]
	fn test_forces() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![(0, 1)],
			Nodes::Degree(2),
			None,
			vec![[-2.0, -2.0], [1.0, 2.0]],
			None,
			Settings::default(),
		);

		layout.init_iteration();
		layout.apply_attraction();

		let speed_1 = dbg!(layout.speeds[0]);
		let speed_2 = dbg!(layout.speeds[1]);

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

		let speed_1 = dbg!(layout.speeds[0]);
		let speed_2 = dbg!(layout.speeds[1]);

		assert!(speed_1[0] < 0.0);
		assert!(speed_1[1] < 0.0);
		assert!(speed_2[0] > 0.0);
		assert!(speed_2[1] > 0.0);
		assert_eq!(speed_1[0], -0.48);
		assert_eq!(speed_1[1], -0.64);
		assert_eq!(speed_2[0], 0.48);
		assert_eq!(speed_2[1], 0.64);

		layout.init_iteration();
		layout.apply_gravity();

		let speed_1 = dbg!(layout.speeds[0]);
		let speed_2 = dbg!(layout.speeds[1]);

		assert!(speed_1[0] > 0.0);
		assert!(speed_1[1] > 0.0);
		assert!(speed_2[0] < 0.0);
		assert!(speed_2[1] < 0.0);
		assert_eq!(speed_1[0], 2.0 / 2.0.sqrt());
		assert_eq!(speed_1[1], 2.0 / 2.0.sqrt());
		assert_eq!(speed_2[0], -2.0 / 5.0.sqrt());
		assert_eq!(speed_2[1], -4.0 / 5.0.sqrt());
	}

	#[test]
	fn test_convergence() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![(0, 1), (1, 2)],
			Nodes::Degree(3),
			None,
			vec![[-1.1, -1.0], [0.0, 0.0], [1.0, 1.0]],
			None,
			Settings {
				dissuade_hubs: false,
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
			println!("{:?}", layout.speeds);
			layout.init_iteration();
			layout.apply_repulsion();
			println!("{:?}", layout.speeds);
			layout.init_iteration();
			layout.apply_gravity();
			println!("{:?}", layout.speeds);
			layout.apply_forces();
			//layout.iteration();

			dbg!(&layout.points);
			let point_1 = layout.points[0];
			let point_2 = layout.points[1];
			dbg!(((point_2[0] - point_1[0]).powi(2) + (point_2[1] - point_1[1]).powi(2)).sqrt());
		}
	}

	#[test]
	fn test_convergence_po() {
		let mut layout = Layout::<f64, 2>::from_position_graph(
			vec![(0, 1), (1, 2)],
			Nodes::Degree(3),
			Some(vec![1.0, 5.0, 1.0]),
			vec![[-1.1, -1.0], [0.0, 0.0], [1.0, 1.0]],
			None,
			Settings {
				dissuade_hubs: false,
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
			println!("{:?}", layout.speeds);
			layout.init_iteration();
			layout.apply_repulsion();
			println!("{:?}", layout.speeds);
			layout.init_iteration();
			layout.apply_gravity();
			println!("{:?}", layout.speeds);
			layout.apply_forces();
			//layout.iteration();

			dbg!(&layout.points);
			let point_1 = layout.points[0];
			let point_2 = layout.points[1];
			dbg!(((point_2[0] - point_1[0]).powi(2) + (point_2[1] - point_1[1]).powi(2)).sqrt());
		}
	}

	#[test]
	fn check_alloc() {
		let mut layout = Layout::<f64, 2>::from_graph(
			vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 4), (3, 4)],
			Nodes::Degree(5),
			None,
			None,
			Settings::default(),
		);

		deny_alloc(|| layout.init_iteration());
		deny_alloc(|| layout.apply_attraction());
		deny_alloc(|| layout.apply_gravity());
		deny_alloc(|| layout.apply_forces());
	}
}
