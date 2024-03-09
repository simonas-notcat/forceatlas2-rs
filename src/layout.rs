use crate::util::*;

#[derive(Clone)]
pub struct Settings<T: Coord> {
	/// Precision setting for Barnes-Hut computation
	///
	/// Must be in `(0.0..1.0)`. `0.0` is accurate and slow, `1.0` is unaccurate and fast.
	/// Default is `0.5`.
	pub theta: T,
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
			theta: T::one() / (T::one() + T::one()),
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

impl<T: Coord> Settings<T> {
	pub fn check(&self) -> bool {
		self.theta >= T::zero() && self.theta <= T::one()
	}
}

pub struct Layout<T: Coord, const N: usize> {
	pub edges: Vec<Edge>,
	pub masses: Vec<T>,
	pub points: Vec<[T; N]>,
	pub sizes: Option<Vec<T>>,
	pub speeds: Vec<[T; N]>,
	pub old_speeds: Vec<[T; N]>,
	pub weights: Option<Vec<T>>,
	// Mutex needed here to be Sync
	pub(crate) bump: parking_lot::Mutex<bumpalo::Bump>,
	pub(crate) fn_attraction: fn(&mut Self),
	pub(crate) fn_gravity: fn(&mut Self),
	pub(crate) fn_repulsion: fn(&mut Self),
	pub(crate) settings: Settings<T>,
}
