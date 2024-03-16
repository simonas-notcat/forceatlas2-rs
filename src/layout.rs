use crate::util::*;

/// Settings for the graph layout
#[derive(Clone)]
pub struct Settings<T> {
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
	/// Check whether the settings are valid
	pub fn check(&self) -> bool {
		self.theta >= T::zero() && self.theta <= T::one()
	}
}

pub struct Node<T, const N: usize> {
	pos: [T; N],
	speed: [T; N],
	old_speed: [T; N],
	size: T,
	mass: T,
}

/// Graph spatialization layout
pub struct Layout<T, const N: usize> {
	pub nodes: Vec<Node>,
	/// Graph edges (undirected)
	pub edges: Vec<Edge>,
	/// Node weights
	pub weights: Option<Vec<T>>,
	// Mutex needed here for Layout to be Sync
	pub(crate) bump: parking_lot::Mutex<bumpalo::Bump>,
	pub(crate) fn_attraction: fn(&mut Self),
	pub(crate) fn_gravity: fn(&mut Self),
	pub(crate) fn_repulsion: fn(&mut Self),
	pub(crate) settings: Settings<T>,
}
