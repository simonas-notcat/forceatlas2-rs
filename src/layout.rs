use crate::util::*;

#[derive(Clone)]
pub struct Settings<T: Coord> {
	/// Optimize repulsion using Barnes-Hut algorithm (time passes from N^2 to NlogN)
	/// The argument is theta.
	///
	/// **Note**: only implemented for `T=f64` and `dimension` 2 or 3.
	pub barnes_hut: T,
	/// Number of nodes computed by each thread
	///
	/// Only used in repulsion computation. Set to `None` to turn off parallelization.
	/// This number should be big enough to minimize thread management,
	/// but small enough to maximize concurrency.
	///
	/// Requires `T: Send + Sync`
	#[cfg(feature = "parallel")]
	pub chunk_size: Option<usize>,
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
			barnes_hut: T::one() / (T::one() + T::one()),
			#[cfg(feature = "parallel")]
			chunk_size: Some(256),
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

pub struct Layout<T: Coord, const N: usize> {
	pub edges: Vec<Edge>,
	pub masses: Vec<T>,
	pub sizes: Option<Vec<T>>,
	/// List of the nodes' positions
	pub points: Vec<[T; N]>,
	pub(crate) settings: Settings<T>,
	pub speeds: Vec<[T; N]>,
	pub old_speeds: Vec<[T; N]>,
	pub weights: Option<Vec<T>>,

	pub(crate) fn_attraction: fn(&mut Self),
	pub(crate) fn_gravity: fn(&mut Self),
	pub(crate) fn_repulsion: fn(&mut Self),
}

/*impl<T: Coord> Layout<T> {
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
}*/
