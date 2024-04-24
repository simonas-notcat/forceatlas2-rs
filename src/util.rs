use num_traits::{cast::FromPrimitive, real::Real, sign::Signed, Num, Zero};
#[cfg(feature = "rand")]
use rand::Rng;
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

/// Some traits that are convenient for coordinates
pub trait Coord:
	AddAssign<Self>
	+ DivAssign<Self>
	+ FromPrimitive
	+ Real
	+ Signed
	+ SubAssign<Self>
	+ MulAssign<Self>
	+ std::iter::Sum
{
}

impl<T> Coord for T where
	T: AddAssign<Self>
		+ DivAssign<Self>
		+ FromPrimitive
		+ Real
		+ Signed
		+ SubAssign<Self>
		+ MulAssign<Self>
		+ std::iter::Sum
{
}

/// Undirected graph edge indexing its two nodes
pub type Edge = (usize, usize);

/// N-dimensional vector
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VecN<T, const N: usize>(pub [T; N]);

impl<T, const N: usize> IntoIterator for VecN<T, N> {
	type Item = T;
	type IntoIter = std::array::IntoIter<T, N>;
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<T, const N: usize> AsRef<[T; N]> for VecN<T, N> {
	fn as_ref(&self) -> &[T; N] {
		&self.0
	}
}

impl<T, const N: usize> AsMut<[T; N]> for VecN<T, N> {
	fn as_mut(&mut self) -> &mut [T; N] {
		&mut self.0
	}
}

impl<T, I, const N: usize> std::ops::Index<I> for VecN<T, N>
where
	[T; N]: std::ops::Index<I>,
{
	type Output = <[T; N] as std::ops::Index<I>>::Output;
	fn index(&self, index: I) -> &Self::Output {
		&self.0[index]
	}
}

impl<T, I, const N: usize> std::ops::IndexMut<I> for VecN<T, N>
where
	[T; N]: std::ops::IndexMut<I>,
{
	fn index_mut(&mut self, index: I) -> &mut <Self as std::ops::Index<I>>::Output {
		&mut self.0[index]
	}
}

impl<T, const N: usize> VecN<T, N> {
	/// Square norm
	pub fn norm_squared(self) -> T
	where
		T: Copy + Num,
	{
		self.0.into_iter().fold(T::zero(), |s, i| s + i * i)
	}

	/// Square distance to rhs
	pub fn distance_squared(self, rhs: Self) -> T
	where
		T: Copy + Num,
	{
		self.0
			.into_iter()
			.zip(rhs.0)
			.fold(T::zero(), |s, (a, b)| s + (a - b) * (a - b))
	}

	/// Distance to rhs
	pub fn distance(self, rhs: Self) -> T
	where
		T: Real,
	{
		self.distance_squared(rhs).sqrt()
	}

	/// Iterate through the vector's coordinates
	pub fn iter(&self) -> std::slice::Iter<T> {
		self.0.iter()
	}

	/// Iterate mutably through the vector's coordinates
	pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
		self.0.iter_mut()
	}
}

impl<T: Copy + Num, const N: usize> std::ops::Add<VecN<T, N>> for VecN<T, N> {
	type Output = Self;
	fn add(mut self, rhs: Self) -> Self::Output {
		self.0
			.iter_mut()
			.zip(rhs.0.iter())
			.for_each(|(a, b)| *a = *a + *b);
		self
	}
}

impl<T: Copy + Num, const N: usize> std::ops::AddAssign<VecN<T, N>> for VecN<T, N> {
	fn add_assign(&mut self, other: Self) {
		self.0
			.iter_mut()
			.zip(other.0.iter())
			.for_each(|(a, b)| *a = *a + *b);
	}
}

impl<T: Copy + Num, const N: usize> std::ops::Sub<VecN<T, N>> for VecN<T, N> {
	type Output = Self;
	fn sub(mut self, rhs: Self) -> Self::Output {
		self.0
			.iter_mut()
			.zip(rhs.0.iter())
			.for_each(|(a, b)| *a = *a - *b);
		self
	}
}

impl<T: Copy + Num, const N: usize> std::ops::SubAssign<VecN<T, N>> for VecN<T, N> {
	fn sub_assign(&mut self, other: Self) {
		self.0
			.iter_mut()
			.zip(other.0.iter())
			.for_each(|(a, b)| *a = *a - *b);
	}
}

impl<T: Copy + Num, const N: usize> std::ops::Mul<T> for VecN<T, N> {
	type Output = Self;
	fn mul(mut self, rhs: T) -> Self::Output {
		self.0.iter_mut().for_each(|a| *a = *a * rhs);
		self
	}
}

impl<T: Copy + Num, const N: usize> std::ops::Div<T> for VecN<T, N> {
	type Output = Self;
	fn div(mut self, rhs: T) -> Self::Output {
		self.0.iter_mut().for_each(|a| *a = *a / rhs);
		self
	}
}

impl<T, const N: usize> std::ops::Neg for VecN<T, N>
where
	T: std::ops::Neg<Output = T> + Copy,
{
	type Output = Self;
	fn neg(mut self) -> Self::Output {
		self.0.iter_mut().for_each(|a| *a = -*a);
		self
	}
}

impl<T: Copy + Num, const N: usize> Zero for VecN<T, N> {
	fn zero() -> Self {
		Self([T::zero(); N])
	}

	fn is_zero(&self) -> bool {
		self.0.iter().all(Zero::is_zero)
	}
}

/// 2D vector
pub type Vec2<T> = VecN<T, 2>;
/// 3D vector
pub type Vec3<T> = VecN<T, 3>;

impl<T> Vec2<T> {
	/// Create 2D vector from coordinates
	pub fn new(x: T, y: T) -> Self {
		Self([x, y])
	}

	/// Get x coordinate
	pub fn x(&self) -> T
	where
		T: Clone,
	{
		self.0[0].clone()
	}

	/// Get y coordinate
	pub fn y(&self) -> T
	where
		T: Clone,
	{
		self.0[1].clone()
	}
}

impl<T> Vec3<T> {
	/// Create 3D vector from coordinates
	pub fn new(x: T, y: T, z: T) -> Self {
		Self([x, y, z])
	}

	/// Get x coordinate
	pub fn x(&self) -> T
	where
		T: Clone,
	{
		self.0[0].clone()
	}

	/// Get y coordinate
	pub fn y(&self) -> T
	where
		T: Clone,
	{
		self.0[1].clone()
	}

	/// Get z coordinate
	pub fn z(&self) -> T
	where
		T: Clone,
	{
		self.0[2].clone()
	}
}

pub(crate) fn get_2_mut<T>(s: &mut [T], i1: usize, i2: usize) -> (&mut T, &mut T) {
	let (s1, s2) = s.split_at_mut(i2);
	(&mut s1[i1], &mut s2[0])
}

/// Uniform random distribution of points in an (N-1)-cube
#[cfg(feature = "rand")]
pub fn sample_unit_cube<T, R: Rng, const N: usize>(rng: &mut R) -> VecN<T, N>
where
	rand::distributions::Standard: rand::distributions::Distribution<T>,
	T: Coord + rand::distributions::uniform::SampleUniform,
{
	let mut v = VecN::<T, N>::zero();
	for x in v.iter_mut() {
		*x = rng.gen_range(T::one().neg()..T::one());
	}
	v
}

/// Keeps only items for which filter returns true.
/// Does not preserve order.
pub fn drain_filter_swap<T, F: Fn(&mut T) -> bool>(v: &mut Vec<T>, filter: F) {
	let mut i = 0;
	while let Some(item) = v.get_mut(i) {
		if (filter)(item) {
			i += 1;
		} else {
			v.swap_remove(i);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_drain_filter_swap() {
		let mut v = vec![1, -2, 3, -4, -5, 6, 7, 8, -9, 10];
		drain_filter_swap(&mut v, |n| *n > 0);
		assert_eq!(&v, &[1, 10, 3, 8, 7, 6]);
	}
}
