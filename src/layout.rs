use crate::{iter::*, util::*};

use rayon::prelude::*;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Settings<T: Coord> {
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
	/// Prevent node overlapping for a prettier graph (node_size, kr_prime).
	///
	/// `node_size` is the radius around a node where the repulsion coefficient is `kr_prime`.
	/// `kr_prime` is arbitrarily set to `100.0` in Gephi implementation.
	pub prevent_overlapping: Option<(T, T)>,
	/// Gravity does not decrease with distance, resulting in a more compact graph.
	pub strong_gravity: bool,
	/// Optimize repulsion using Barnes-Hut algorithm (time passes from N^2 to NlogN).
	/// The argument is theta.
	///
	/// **Note**: only implemented for `T=f64` and `dimension` 2 or 3.
	#[cfg(feature = "barnes_hut")]
	pub barnes_hut: Option<T>,
}

impl<T: Coord> Default for Settings<T> {
	fn default() -> Self {
		Self {
			dimensions: 2,
			dissuade_hubs: false,
			ka: T::one(),
			kg: T::one(),
			kr: T::one(),
			lin_log: false,
			prevent_overlapping: None,
			strong_gravity: false,
			#[cfg(feature = "barnes_hut")]
			barnes_hut: None,
		}
	}
}

pub struct Layout<T: Coord> {
	pub edges: Vec<Edge>,
	pub masses: Vec<T>,
	/// List of the nodes' positions
	pub points: PointList<T>,
	pub(crate) settings: Settings<T>,
	pub speeds: PointList<T>,
	pub old_speeds: PointList<T>,

	pub(crate) fn_attraction: fn(&mut Self),
	pub(crate) fn_gravity: fn(&mut Self),
	pub(crate) fn_repulsion: fn(&mut Self),
}

impl<T: Coord> Layout<T> {
	pub fn iter_nodes(&mut self) -> NodeIter<T> {
		NodeIter {
			layout: SendPtr(self.into()),
			offset: 0,
			_phantom: PhantomData::default(),
		}
	}
}

impl<T: Coord + Send> Layout<T> {
	pub fn iter_par_nodes(
		&mut self,
		chunk_size: usize,
	) -> impl Iterator<Item = impl ParallelIterator<Item = NodeParIter<T>>> {
		let ptr = SendPtr(self.into());
		let dimensions = self.settings.dimensions;
		let chunk_size_d = chunk_size * dimensions;
		(0..self.masses.len()).step_by(chunk_size).map(move |y0| {
			let y0_d = y0 * dimensions;
			(0..self.masses.len() - y0)
				.into_par_iter()
				.step_by(chunk_size)
				.map(move |x0| {
					let x0_d = x0 * dimensions;
					NodeParIter {
						end: x0_d + chunk_size_d,
						layout: ptr,
						n2_start: x0_d + y0_d,
						n2_end: x0_d + y0_d + chunk_size_d,
						offset: x0_d,
						_phantom: PhantomData::default(),
					}
				})
		})
	}
}
