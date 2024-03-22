use crate::util::*;

use num_traits::{real::Real, Zero};
use std::borrow::{Borrow, BorrowMut};

/// Maximum tree depth
const MAX_DEPTH: usize = 32;

pub struct Tree<'a, D, T: 'a, C, L: 'a, const N: usize> {
	bump: &'a mut bumpalo::Bump,
	phantom: std::marker::PhantomData<([T; N], C, L, &'a D)>,
}

impl<'a, T, C, L, D: Node<T, C, L, N>, const N: usize> Tree<'a, D, T, C, L, N> {
	/// Create an empty tree
	pub fn from_bump(bump: &'a mut bumpalo::Bump) -> Self {
		Tree {
			bump,
			phantom: Default::default(),
		}
	}

	pub fn new_root<'r>(&'r mut self, pos: (VecN<T, N>, VecN<T, N>)) -> Root<'r, D, T, C, L, N>
	where
		'a: 'r,
	{
		Root {
			node: bumpalo::boxed::Box::new_in(D::new(pos), self.bump),
			phantom: Default::default(),
		}
	}

	pub fn clear(&mut self) {
		self.bump.reset();
	}
}

pub struct Root<'r, D, T, C, L, const N: usize> {
	node: bumpalo::boxed::Box<'r, D>,
	phantom: std::marker::PhantomData<(T, C, [L; N])>,
}

impl<'r, D: Node<T, C, L, N>, T, C, L, const N: usize> Root<'r, D, T, C, L, N> {
	pub fn add_body(&mut self, new_body: L) {
		BorrowMut::<D>::borrow_mut(&mut self.node).add_body(new_body, 0)
	}
	pub fn apply<
		F1: Fn(VecN<T, N>, VecN<T, N>, T, T, C) -> VecN<T, N>,
		F2: Fn(VecN<T, N>, VecN<T, N>, T, T, C, C) -> VecN<T, N>,
	>(
		&self,
		on: VecN<T, N>,
		theta: T,
		custom: C,
		f1: &F1,
		f2: &F2,
	) -> VecN<T, N> {
		Borrow::<D>::borrow(&self.node).apply(on, theta, custom, f1, f2)
	}
}

/// A body in the Barnes-Hut simulation
pub trait Body<T, C, const N: usize> {
	/// Get mass
	fn mass(&self) -> T;
	/// Get position
	fn pos(&self) -> VecN<T, N>;
	/// Change center of mass by adding given mass at given position
	fn add_mass(&mut self, mass: T, pos: VecN<T, N>);
	/// Get custom value
	fn custom(&self) -> C;
}

/// A tree node in the Barnes-Hut simulation
pub trait Node<T, C, L, const N: usize> {
	/// Create a node with given AABB points
	fn new(pos: (VecN<T, N>, VecN<T, N>)) -> Self;
	/// Add a body to the node, at a given recursion depth
	fn add_body(&mut self, new_body: L, depth: usize);
	/// Compute the force applied on a virtual body at a given position
	fn apply<
		F1: Fn(VecN<T, N>, VecN<T, N>, T, T, C) -> VecN<T, N>,
		F2: Fn(VecN<T, N>, VecN<T, N>, T, T, C, C) -> VecN<T, N>,
	>(
		&self,
		on: VecN<T, N>,
		theta: T,
		custom: C,
		f1: &F1,
		f2: &F2,
	) -> VecN<T, N>;
}

pub enum NodeN<T, L, const N: usize, const NP: usize> {
	Branch {
		nodes: Box<[NodeN<T, L, N, NP>; NP]>,
		center: VecN<T, N>,
		mass: T,
		center_of_mass: VecN<T, N>,
		width: T,
	},
	Leaf {
		body: Option<L>,
		pos: (VecN<T, N>, VecN<T, N>),
	},
}

pub type Node2<T, L> = NodeN<T, L, 2, 4>;
pub type Node3<T, L> = NodeN<T, L, 3, 8>;

impl<T: Real, C: Clone, L: Body<T, C, 2>> Node<T, C, L, 2> for Node2<T, L> {
	fn new(pos: (Vec2<T>, Vec2<T>)) -> Self {
		Node2::Leaf { body: None, pos }
	}
	fn add_body(&mut self, new_body: L, depth: usize) {
		match self {
			Node2::Branch {
				nodes,
				center,
				mass,
				center_of_mass,
				..
			} => {
				let new_body_pos = new_body.pos();
				let new_body_mass = new_body.mass();

				*center_of_mass = (*center_of_mass * *mass + new_body_pos * new_body_mass)
					/ (*mass + new_body_mass);
				*mass = *mass + new_body_mass;
				nodes[match (new_body_pos.x() < center.x(), new_body_pos.y() < center.y()) {
					(true, true) => 0,
					(false, true) => 1,
					(true, false) => 2,
					(false, false) => 3,
				}]
				.add_body(new_body, depth + 1)
			}
			Node2::Leaf { body, pos } => {
				if let Some(mut body) = body.take() {
					if depth > MAX_DEPTH || body.pos().distance_squared(new_body.pos()) < T::one() {
						body.add_mass(new_body.mass(), new_body.pos());
						*self = Node2::Leaf {
							body: Some(body),
							pos: *pos,
						};
						return;
					}
					let center = (pos.0 + pos.1) / (T::one() + T::one());
					*self = Node2::Branch {
						nodes: Box::new([
							Node2::Leaf {
								body: None,
								pos: (pos.0, center),
							},
							Node2::Leaf {
								body: None,
								pos: (
									Vec2::new(center.x(), pos.0.y()),
									Vec2::new(pos.1.x(), center.y()),
								),
							},
							Node2::Leaf {
								body: None,
								pos: (
									Vec2::new(pos.0.x(), center.y()),
									Vec2::new(center.x(), pos.1.y()),
								),
							},
							Node2::Leaf {
								body: None,
								pos: (center, pos.1),
							},
						]),
						center,
						mass: T::zero(),
						center_of_mass: center,
						width: pos.1.x() - pos.0.x(),
					};
					self.add_body(body, depth + 1);
					self.add_body(new_body, depth + 1)
				} else {
					*body = Some(new_body);
				}
			}
		}
	}
	fn apply<
		F1: Fn(Vec2<T>, Vec2<T>, T, T, C) -> Vec2<T>,
		F2: Fn(Vec2<T>, Vec2<T>, T, T, C, C) -> Vec2<T>,
	>(
		&self,
		on: Vec2<T>,
		theta: T,
		custom: C,
		f1: &F1,
		f2: &F2,
	) -> Vec2<T> {
		match self {
			Node2::Branch {
				nodes,
				mass,
				center_of_mass,
				width,
				..
			} => {
				if on == *center_of_mass {
					return Zero::zero();
				}
				let dist = on.distance(*center_of_mass);
				if *width / dist < theta {
					f1(*center_of_mass, on, *mass, dist, custom)
				} else {
					nodes[0].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[1].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[2].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[3].apply::<F1, F2>(on, theta, custom, f1, f2)
				}
			}
			Node2::Leaf { body, .. } => {
				if let Some(body) = body {
					if on == body.pos() {
						return Zero::zero();
					}
					let dist = on.distance(body.pos());
					f2(body.pos(), on, body.mass(), dist, custom, body.custom())
				} else {
					Zero::zero()
				}
			}
		}
	}
}

impl<T: Real, C: Clone, L: Body<T, C, 3>> Node<T, C, L, 3> for Node3<T, L> {
	fn new(pos: (Vec3<T>, Vec3<T>)) -> Self {
		Node3::Leaf { body: None, pos }
	}
	fn add_body(&mut self, new_body: L, depth: usize) {
		match self {
			Node3::Branch {
				nodes,
				center,
				mass,
				center_of_mass,
				..
			} => {
				let new_body_pos = new_body.pos();
				let new_body_mass = new_body.mass();

				*center_of_mass = (*center_of_mass * *mass + new_body_pos * new_body_mass)
					/ (*mass + new_body_mass);
				*mass = *mass + new_body_mass;
				nodes[match (
					new_body_pos.x() < center.x(),
					new_body_pos.y() < center.y(),
					new_body_pos.z() < center.z(),
				) {
					(true, true, true) => 0,
					(false, true, true) => 1,
					(true, false, true) => 2,
					(false, false, true) => 3,
					(true, true, false) => 4,
					(false, true, false) => 5,
					(true, false, false) => 6,
					(false, false, false) => 7,
				}]
				.add_body(new_body, depth + 1)
			}
			Node3::Leaf { body, pos } => {
				if let Some(mut body) = body.take() {
					if depth > MAX_DEPTH || body.pos().distance_squared(new_body.pos()) < T::one() {
						body.add_mass(new_body.mass(), new_body.pos());
						*self = Node3::Leaf {
							body: Some(body),
							pos: *pos,
						};
						return;
					}
					let center = (pos.0 + pos.1) / (T::one() + T::one());
					*self = Node3::Branch {
						nodes: Box::new([
							Node3::Leaf {
								body: None,
								pos: (pos.0, center),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(center.x(), pos.0.y(), pos.0.z()),
									Vec3::new(pos.1.x(), center.y(), center.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(pos.0.x(), center.y(), pos.0.z()),
									Vec3::new(center.x(), pos.1.y(), center.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(center.x(), center.y(), pos.0.z()),
									Vec3::new(pos.1.x(), pos.1.y(), center.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(pos.0.x(), pos.0.y(), center.z()),
									Vec3::new(center.x(), center.y(), pos.1.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(center.x(), pos.0.y(), center.z()),
									Vec3::new(pos.1.x(), center.y(), pos.1.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (
									Vec3::new(pos.0.x(), center.y(), center.z()),
									Vec3::new(center.x(), pos.1.y(), pos.1.z()),
								),
							},
							Node3::Leaf {
								body: None,
								pos: (center, pos.1),
							},
						]),
						center,
						mass: T::zero(),
						center_of_mass: center,
						width: pos.1.x() - pos.0.x(),
					};
					self.add_body(body, depth + 1);
					self.add_body(new_body, depth + 1)
				} else {
					*body = Some(new_body);
				}
			}
		}
	}
	fn apply<
		F1: Fn(Vec3<T>, Vec3<T>, T, T, C) -> Vec3<T>,
		F2: Fn(Vec3<T>, Vec3<T>, T, T, C, C) -> Vec3<T>,
	>(
		&self,
		on: Vec3<T>,
		theta: T,
		custom: C,
		f1: &F1,
		f2: &F2,
	) -> Vec3<T> {
		match self {
			Node3::Branch {
				nodes,
				mass,
				center_of_mass,
				width,
				..
			} => {
				if on == *center_of_mass {
					return Zero::zero();
				}
				let dist = on.distance(*center_of_mass);
				if *width / dist < theta {
					f1(*center_of_mass, on, *mass, dist, custom)
				} else {
					nodes[0].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[1].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[2].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[3].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[4].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[5].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[6].apply::<F1, F2>(on, theta, custom.clone(), f1, f2)
						+ nodes[7].apply::<F1, F2>(on, theta, custom, f1, f2)
				}
			}
			Node3::Leaf { body, .. } => {
				if let Some(body) = body {
					if on == body.pos() {
						return Zero::zero();
					}
					let dist = on.distance(body.pos());
					f2(body.pos(), on, body.mass(), dist, custom, body.custom())
				} else {
					Zero::zero()
				}
			}
		}
	}
}
