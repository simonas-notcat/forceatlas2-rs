use crate::util::*;

/// Settings for the graph layout
#[derive(Clone, Debug)]
pub struct Settings<T> {
	/// Precision setting for Barnes-Hut computation
	///
	/// Must be in `(0.0..1.0)`. `0.0` is accurate and slow, `1.0` is unaccurate and fast.
	/// Default is `0.5`.
	pub theta: T,
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

#[derive(Clone, Debug)]
pub struct Node<T, const N: usize> {
	pub pos: [T; N],
	pub speed: [T; N],
	pub old_speed: [T; N],
	pub size: T,
	pub mass: T,
}

impl<T: Coord, const N: usize> Default for Node<T, N> {
	fn default() -> Self {
		Node {
			pos: [T::zero(); N],
			speed: [T::zero(); N],
			old_speed: [T::zero(); N],
			size: T::one(),
			mass: T::one(),
		}
	}
}

/// Graph spatialization layout
pub struct Layout<T, const N: usize> {
	pub nodes: Vec<Node<T, N>>,
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
