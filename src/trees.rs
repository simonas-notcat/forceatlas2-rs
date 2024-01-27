use std::borrow::{Borrow, BorrowMut};

use num_traits::{real::Real, Num, Zero};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VecN<T, const N: usize>([T; N]);

impl<T, const N: usize> VecN<T, N> {
	fn _norm_squared(self) -> T
	where
		T: Copy + Num,
	{
		self.0.into_iter().fold(T::zero(), |s, i| s + i * i)
	}

	fn distance_squared(self, rhs: Self) -> T
	where
		T: Copy + Num,
	{
		self.0
			.into_iter()
			.zip(rhs.0.into_iter())
			.fold(T::zero(), |s, (a, b)| s + (a - b) * (a - b))
	}

	fn distance(self, rhs: Self) -> T
	where
		T: Real,
	{
		self.distance_squared(rhs).sqrt()
	}
}

impl<T, const N: usize> std::ops::Add<VecN<T, N>> for VecN<T, N>
where
	T: Num + Copy,
{
	type Output = Self;
	fn add(mut self, rhs: Self) -> Self::Output {
		self.0
			.iter_mut()
			.zip(rhs.0.iter())
			.for_each(|(a, b)| *a = *a + *b);
		self
	}
}

impl<T, const N: usize> Zero for VecN<T, N>
where
	T: Copy + Num,
{
	fn zero() -> Self {
		Self([T::zero(); N])
	}

	fn is_zero(&self) -> bool {
		self.0.iter().all(Zero::is_zero)
	}
}

pub type Vec2<T> = VecN<T, 2>;

impl<T> std::ops::Sub<Vec2<T>> for Vec2<T>
where
	T: Num + Copy,
{
	type Output = Self;
	fn sub(self, rhs: Self) -> Self {
		Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
	}
}

impl<T> std::ops::Mul<T> for Vec2<T>
where
	T: Num + Copy,
{
	type Output = Self;
	fn mul(self, rhs: T) -> Self {
		Self([self.0[0] * rhs, self.0[1] * rhs])
	}
}

impl<T> std::ops::Div<T> for Vec2<T>
where
	T: Num + Copy,
{
	type Output = Self;
	fn div(self, rhs: T) -> Self {
		Self([self.0[0] / rhs, self.0[1] / rhs])
	}
}

impl<T> Vec2<T> {
	pub fn new(x: T, y: T) -> Self {
		Self([x, y])
	}

	pub fn x(&self) -> T
	where
		T: Clone,
	{
		self.0[0].clone()
	}

	pub fn y(&self) -> T
	where
		T: Clone,
	{
		self.0[1].clone()
	}
}

pub struct Tree<'a, D, T: 'a, L: 'a, const N: usize> {
	bump: bumpalo::Bump,
	phantom: std::marker::PhantomData<([T; N], L, &'a D)>,
}

impl<'a, T, L, D: Node<T, L, N>, const N: usize> Tree<'a, D, T, L, N> {
	/// Create an empty tree with preallocated space for a given number of nodes
	pub fn with_capacity(capacity: usize) -> Self {
		let bump = bumpalo::Bump::with_capacity(std::mem::size_of::<D>() * capacity);
		Tree {
			bump,
			phantom: Default::default(),
		}
	}

	pub fn new_root<'r>(&'r mut self, pos: (VecN<T, N>, VecN<T, N>)) -> Root<'r, D, T, L, N>
	where
		'a: 'r,
	{
		Root {
			node: bumpalo::boxed::Box::new_in(D::new(pos), &self.bump),
			p: Default::default(),
		}
	}

	pub fn clear(&mut self) {
		self.bump.reset();
	}
}

pub struct Root<'a, D, T: 'a, L: 'a, const N: usize> {
	node: bumpalo::boxed::Box<'a, D>,
	p: std::marker::PhantomData<(T, [L; N])>,
}

impl<'a, D: Node<T, L, N>, T: 'a, L: 'a, const N: usize> Root<'a, D, T, L, N> {
	pub fn add_body(&mut self, new_body: L) {
		BorrowMut::<D>::borrow_mut(&mut self.node).add_body(new_body)
	}
	pub fn apply<C: Clone, F: Fn(VecN<T, N>, VecN<T, N>, T, T, C) -> VecN<T, N>>(
		&self,
		on: VecN<T, N>,
		theta: T,
		custom: C,
		f: &F,
	) -> VecN<T, N> {
		Borrow::<D>::borrow(&self.node).apply(on, theta, custom, f)
	}
}

pub trait Body<T, const N: usize> {
	fn mass(&self) -> T;
	fn pos(&self) -> VecN<T, N>;
	fn add_mass(&mut self, mass: T);
}

pub trait Node<T, L, const N: usize> {
	fn new(pos: (VecN<T, N>, VecN<T, N>)) -> Self;
	fn add_body(&mut self, new_body: L);
	fn apply<C: Clone, F: Fn(VecN<T, N>, VecN<T, N>, T, T, C) -> VecN<T, N>>(
		&self,
		on: VecN<T, N>,
		theta: T,
		custom: C,
		f: &F,
	) -> VecN<T, N>;
}

pub enum Node2<T, L> {
	Branch {
		nodes: Box<[Node2<T, L>; 4]>,
		center: Vec2<T>,
		mass: T,
		center_of_mass: Vec2<T>,
		width: T,
	},
	Leaf {
		body: Option<L>,
		pos: (Vec2<T>, Vec2<T>),
	},
}

impl<T, L: Body<T, 2>> Node<T, L, 2> for Node2<T, L>
where
	T: Real,
{
	fn new(pos: (Vec2<T>, Vec2<T>)) -> Self {
		Node2::Leaf { body: None, pos }
	}
	fn add_body(&mut self, new_body: L) {
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
				.add_body(new_body)
			}
			Node2::Leaf { body, pos } => {
				if let Some(mut body) = body.take() {
					if body.pos().distance_squared(new_body.pos()) < T::one() {
						body.add_mass(new_body.mass());
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
					self.add_body(body);
					self.add_body(new_body)
				} else {
					*body = Some(new_body);
				}
			}
		}
	}
	fn apply<C: Clone, F: Fn(Vec2<T>, Vec2<T>, T, T, C) -> Vec2<T>>(
		&self,
		on: Vec2<T>,
		theta: T,
		custom: C,
		f: &F,
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
					f(*center_of_mass, on, *mass, dist, custom)
				} else {
					nodes[0].apply::<C, F>(on, theta, custom.clone(), f)
						+ nodes[1].apply::<C, F>(on, theta, custom.clone(), f)
						+ nodes[2].apply::<C, F>(on, theta, custom.clone(), f)
						+ nodes[3].apply::<C, F>(on, theta, custom.clone(), f)
				}
			}
			Node2::Leaf { body, .. } => {
				if let Some(body) = body {
					if on == body.pos() {
						return Zero::zero();
					}
					let dist = on.distance(body.pos());
					f(body.pos(), on, body.mass(), dist, custom)
				} else {
					Zero::zero()
				}
			}
		}
	}
}