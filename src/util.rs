use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

use num_traits::{
	cast::{FromPrimitive, NumCast},
	real::Real,
	sign::Signed,
};
#[cfg(feature = "rand")]
use rand::Rng;

pub trait Coord = AddAssign<Self>
	+ DivAssign<Self>
	+ FromPrimitive
	+ Real
	+ NumCast
	+ Signed
	+ SubAssign<Self>
	+ MulAssign<Self>
	+ std::iter::Sum;

pub type Edge = (usize, usize);

pub enum Nodes<T> {
	Mass(Vec<T>),
	Degree(usize),
}

pub(crate) fn get_2_mut<T>(s: &mut [T], i1: usize, i2: usize) -> (&mut T, &mut T) {
	let (s1, s2) = s.split_at_mut(i2);
	(&mut s1[i1], &mut s2[0])
}

/// Uniform random distribution of points on a n-sphere
///
/// `n` is the number of spatial dimensions (1 => two points; 2 => circle; 3 => sphere; etc.).
#[cfg(feature = "rand")]
pub fn _sample_unit_nsphere<T: Coord, R: Rng, const N: usize>(rng: &mut R) -> [T; N]
where
	rand::distributions::Standard: rand::distributions::Distribution<T>,
	T: rand::distributions::uniform::SampleUniform,
{
	let ray: T = NumCast::from(N).unwrap();
	let mut v = [T::zero(); N];
	let mut d = T::zero();
	for x in v.iter_mut() {
		*x = rng.gen_range(ray.neg()..ray);
		d += *x * *x;
	}
	d = d.sqrt();
	for x in v.iter_mut() {
		*x /= d;
	}
	v
}

/// Uniform random distribution of points in a n-cube
///
/// `n` is the number of spatial dimensions (1 => segment; 2 => square; 3 => cube; etc.).
#[cfg(feature = "rand")]
pub fn sample_unit_ncube<T: Coord, R: Rng, const N: usize>(rng: &mut R) -> [T; N]
where
	rand::distributions::Standard: rand::distributions::Distribution<T>,
	T: rand::distributions::uniform::SampleUniform,
{
	let ray: T = NumCast::from(N).unwrap();
	let mut v = [T::zero(); N];
	for x in v.iter_mut() {
		*x = rng.gen_range(ray.neg()..ray);
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
