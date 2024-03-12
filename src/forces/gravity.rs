use crate::{layout::Layout, util::*};

use rayon::prelude::*;

pub fn apply_gravity<T: Coord + Send + Sync, const N: usize>(layout: &mut Layout<T, N>) {
	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((pos, speed), mass)| {
			let mut d = T::zero();
			for x in pos {
				d += *x * *x;
			}
			if d.is_zero() {
				return;
			}
			let f = (*mass + T::one()) * layout.settings.kg / d;
			for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
				*speed -= f * *pos;
			}
		})
}

pub fn apply_gravity_sg<T: Coord + Send + Sync, const N: usize>(layout: &mut Layout<T, N>) {
	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((pos, speed), mass)| {
			let f = (*mass + T::one()) * layout.settings.kg;
			for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
				*speed -= f * *pos;
			}
		})
}
