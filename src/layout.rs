use crate::{iter::*, util::*};

use rayon::prelude::*;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Settings<T: Coord> {
	/// Optimize repulsion using Barnes-Hut algorithm (time passes from N^2 to NlogN)
	/// The argument is theta.
	///
	/// **Note**: only implemented for `T=f64` and `dimension` 2 or 3.
	#[cfg(feature = "barnes_hut")]
	pub barnes_hut: Option<T>,
	/// Number of nodes computed by each thread
	///
	/// Only used in repulsion computation. Set to `None` to turn off parallelization.
	/// This number should be big enough to minimize thread management,
	/// but small enough to maximize concurrency.
	///
	/// Requires `T: Send + Sync`
	#[cfg(feature = "parallel")]
	pub chunk_size: Option<usize>,
	/// Number of spatial dimensions
	pub dimensions: usize,
	/// Move hubs (high degree nodes) to the center
	pub dissuade_hubs: bool,
	/// Attraction coefficient
	pub ka: T,
	/// Gravity coefficient
	pub kg: T,
	/// Repulsion coefficient
	pub kr: T,
	/// Logarithmic attraction
	pub lin_log: bool,
	/// Prevent node overlapping for a prettier graph.
	///
	/// Value is `kr_prime`.
	/// Requires `layout.sizes` to be `Some`.
	/// `kr_prime` is arbitrarily set to `100.0` in Gephi implementation.
	pub prevent_overlapping: Option<T>,
	/// Speed factor
	pub speed: T,
	/// Gravity does not decrease with distance, resulting in a more compact graph.
	pub strong_gravity: bool,
}

impl<T: Coord> Default for Settings<T> {
	fn default() -> Self {
		Self {
			#[cfg(feature = "barnes_hut")]
			barnes_hut: None,
			#[cfg(feature = "parallel")]
			chunk_size: Some(256),
			dimensions: 2,
			dissuade_hubs: false,
			ka: T::one(),
			kg: T::one(),
			kr: T::one(),
			lin_log: false,
			prevent_overlapping: None,
			speed: T::from(0.01).unwrap_or_else(T::one),
			strong_gravity: false,
		}
	}
}

pub struct Layout<T: Coord> {
	pub edges: Vec<Edge>,
	pub masses: Vec<T>,
	pub sizes: Option<Vec<T>>,
	/// List of the nodes' positions
	pub points: PointList<T>,
	pub(crate) settings: Settings<T>,
	pub speeds: PointList<T>,
	pub old_speeds: PointList<T>,
	pub weights: Option<Vec<T>>,

	pub(crate) fn_attraction: fn(&mut Self),
	pub(crate) fn_gravity: fn(&mut Self),
	pub(crate) fn_repulsion: fn(&mut Self),
}

impl<T: Coord> Layout<T> {
	pub fn iter_nodes(&mut self) -> NodeIter<T> {
		NodeIter {
			ind: 0,
			layout: SendPtr(self.into()),
			offset: 0,
			_phantom: PhantomData,
		}
	}
}

#[cfg(feature = "parallel")]
impl<T: Coord + Send> Layout<T> {
	pub fn iter_par_nodes(
		&mut self,
		chunk_size: usize,
	) -> impl Iterator<Item = impl ParallelIterator<Item = NodeParIter<T>>> {
		let ptr = SendPtr(self.into());
		let dimensions = self.settings.dimensions;
		let chunk_size_d = chunk_size * dimensions;
		let n = self.masses.len() * dimensions;
		(0..self.masses.len()).step_by(chunk_size).map(move |y0| {
			let y0_d = y0 * dimensions;
			(0..self.masses.len() - y0)
				.into_par_iter()
				.step_by(chunk_size)
				.map(move |x0| {
					let x0_d = x0 * dimensions;
					NodeParIter {
						end: (x0_d + chunk_size_d).min(n),
						ind: x0,
						layout: ptr,
						n2_start: x0_d + y0_d,
						n2_start_ind: x0 + y0,
						n2_end: (x0_d + y0_d + chunk_size_d).min(n),
						offset: x0_d,
						_phantom: PhantomData,
					}
				})
		})
	}
}

#[cfg(all(feature = "parallel", any(target_arch = "x86", target_arch = "x86_64")))]
impl<T: Coord + Send> Layout<T> {
	pub fn iter_par_simd_nodes<const N: usize>(
		&mut self,
		chunk_size: usize,
	) -> impl Iterator<Item = impl ParallelIterator<Item = NodeParSimdIter<T, N>>> {
		let ptr = SendPtr(self.into());
		let dimensions = self.settings.dimensions;
		let chunk_size_d = chunk_size * dimensions;
		let n = self.masses.len();
		let n_d = n * dimensions;
		(0..n).step_by(chunk_size).map(move |y0| {
			let y0_d = y0 * dimensions;
			(0..n - y0)
				.into_par_iter()
				.step_by(chunk_size)
				.map(move |x0| {
					let x0_d = x0 * dimensions;
					NodeParSimdIter {
						end: (x0_d + chunk_size_d).min(n_d),
						ind: x0,
						layout: ptr,
						n2_start: x0_d + y0_d,
						n2_start_ind: x0 + y0,
						n2_end: (x0_d + y0_d + chunk_size_d).min(n_d),
						n2_end_ind: (x0 + y0 + chunk_size).min(n),
						offset: x0_d,
						_phantom: PhantomData,
					}
				})
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use itertools::iproduct;
	use std::collections::BTreeSet;
	#[cfg(feature = "parallel")]
	use std::sync::{Arc, RwLock};

	#[test]
	fn test_iter_nodes() {
		for n_nodes in 1usize..16 {
			let mut layout = Layout::<f32>::from_graph(
				vec![],
				Nodes::Degree(n_nodes),
				None,
				None,
				Settings::default(),
			);
			let mut hits = iproduct!(0..n_nodes, 0..n_nodes)
				.filter(|(n1, n2)| n1 < n2)
				.collect::<BTreeSet<(usize, usize)>>();
			let points = layout.points.clone();
			for n1 in layout.iter_nodes() {
				for n2 in n1.n2_iter {
					assert!(hits.remove(&(n1.ind, n2.ind)));
					assert_eq!(n1.pos, points.get(n1.ind));
					assert_eq!(n2.pos, points.get(n2.ind));
				}
			}
			assert!(hits.is_empty());
		}
	}

	#[test]
	#[cfg(feature = "parallel")]
	fn test_iter_par_nodes() {
		for n_nodes in 1usize..16 {
			let mut layout = Layout::<f32>::from_graph(
				vec![],
				Nodes::Mass((1..n_nodes + 1).map(|i| i as f32).collect()),
				None,
				None,
				Settings::default(),
			);
			layout
				.speeds
				.iter_mut()
				.enumerate()
				.for_each(|(i, speed)| speed.iter_mut().for_each(|speed| *speed = i as f32));
			let hits = Arc::new(RwLock::new(
				iproduct!(0..n_nodes, 0..n_nodes)
					.filter(|(n1, n2)| n1 < n2)
					.collect::<BTreeSet<(usize, usize)>>(),
			));
			let points = layout.points.clone();
			let speeds = layout.speeds.clone();
			for chunk_iter in layout.iter_par_nodes(4) {
				chunk_iter.for_each(|n1_iter| {
					for n1 in n1_iter {
						for n2 in n1.n2_iter {
							let mut hits = hits.write().unwrap();
							assert!(hits.remove(&(n1.ind, n2.ind)));
							assert_eq!(n1.pos, points.get(n1.ind));
							assert_eq!(n2.pos, points.get(n2.ind));
							assert_eq!(n1.speed, speeds.get(n1.ind));
							assert_eq!(n2.speed, speeds.get(n2.ind));
							assert_eq!(*n1.mass, n1.ind as f32 + 1.);
							assert_eq!(*n2.mass, n2.ind as f32 + 1.);
						}
					}
				});
			}
			assert!(hits.read().unwrap().is_empty());
		}
	}

	#[test]
	#[cfg(feature = "parallel")]
	fn test_iter_par_simd_nodes() {
		rayon::ThreadPoolBuilder::new()
			.num_threads(1)
			.build_global()
			.ok();

		for n_nodes in 1usize..32 {
			println!("######## {} nodes", n_nodes);
			let mut layout = Layout::<f32>::from_graph(
				vec![],
				Nodes::Mass((1..n_nodes + 1).map(|i| i as f32).collect()),
				None,
				None,
				Settings::default(),
			);
			layout
				.speeds
				.iter_mut()
				.enumerate()
				.for_each(|(i, speed)| speed.iter_mut().for_each(|speed| *speed = i as f32));
			let hits = Arc::new(RwLock::new(
				iproduct!(0..n_nodes, 0..n_nodes)
					.filter(|(n1, n2)| n1 < n2)
					.collect::<BTreeSet<(usize, usize)>>(),
			));
			let points = layout.points.clone();
			let speeds = layout.speeds.clone();
			for (level, chunk_iter) in layout.iter_par_simd_nodes::<4>(16).enumerate() {
				println!("level {}", level);
				chunk_iter.for_each(|n1_iter| {
					let n2_end = n1_iter.n2_end_ind;
					for mut n1 in n1_iter {
						println!("n1 {}", n1.ind);
						for n2 in &mut n1.n2_iter {
							let mut hits = hits.write().unwrap();
							println!("d {} {}", n1.ind, n2.ind);
							assert!(hits.remove(&(n1.ind, n2.ind)));
							assert!(hits.remove(&(n1.ind, n2.ind + 1)));
							assert!(hits.remove(&(n1.ind, n2.ind + 2)));
							assert!(hits.remove(&(n1.ind, n2.ind + 3)));
							assert_eq!(n1.pos, points.get(n1.ind));
							unsafe {
								assert_eq!(*n2.pos, points.get(n2.ind)[0]);
								assert_eq!(*n2.pos.add(1), points.get(n2.ind)[1]);
								assert_eq!(*n2.pos.add(2), *points.get(n2.ind).get_unchecked(2));
								assert_eq!(*n2.pos.add(3), *points.get(n2.ind).get_unchecked(3));
								assert_eq!(*n2.pos.add(4), *points.get(n2.ind).get_unchecked(4));
								assert_eq!(*n2.pos.add(5), *points.get(n2.ind).get_unchecked(5));
								assert_eq!(*n2.pos.add(6), *points.get(n2.ind).get_unchecked(6));
								assert_eq!(*n2.pos.add(7), *points.get(n2.ind).get_unchecked(7));
								assert_eq!(*n1.speed, speeds.get(n1.ind)[0]);
								assert_eq!(*n1.speed.add(1), speeds.get(n1.ind)[1]);
								assert_eq!(*n2.speed, speeds.get(n2.ind)[0]);
								assert_eq!(*n2.speed.add(1), speeds.get(n2.ind)[1]);
								assert_eq!(*n2.speed.add(2), *speeds.get(n2.ind).get_unchecked(2));
								assert_eq!(*n2.speed.add(3), *speeds.get(n2.ind).get_unchecked(3));
								assert_eq!(*n2.speed.add(4), *speeds.get(n2.ind).get_unchecked(4));
								assert_eq!(*n2.speed.add(5), *speeds.get(n2.ind).get_unchecked(5));
								assert_eq!(*n2.speed.add(6), *speeds.get(n2.ind).get_unchecked(6));
								assert_eq!(*n2.speed.add(7), *speeds.get(n2.ind).get_unchecked(7));
								assert_eq!(*n1.mass, n1.ind as f32 + 1.);
								assert_eq!(*n2.mass, n2.ind as f32 + 1.);
								assert_eq!(*n2.mass.add(1), n2.ind as f32 + 2.);
								assert_eq!(*n2.mass.add(2), n2.ind as f32 + 3.);
								assert_eq!(*n2.mass.add(3), n2.ind as f32 + 4.);
							}
						}

						println!("d2 {} {}", n1.n2_iter.ind, n1.ind);
						println!(
							"sub {} - {} = {}",
							n1.n2_iter.ind,
							n1.ind,
							n1.n2_iter.ind - n1.ind
						);
						for n2 in n1.n2_iter.ind..n2_end {
							let mut hits = hits.write().unwrap();
							println!("rem {} {}", n1.ind, n2);
							assert!(hits.remove(&(n1.ind, n2)));
						}
					}
				});
			}
			println!("{:?}", hits);
			assert!(hits.read().unwrap().is_empty());
		}
	}
}
