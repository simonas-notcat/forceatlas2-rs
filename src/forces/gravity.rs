use crate::{layout::Layout, util::*};

pub fn apply_gravity<T: Coord + std::fmt::Debug, const N: usize>(layout: &mut Layout<T, N>) {
	for ((mass, pos), speed) in layout
		.masses
		.iter()
		.zip(layout.points.iter())
		.zip(layout.speeds.iter_mut())
	{
		let mut d = T::zero();
		for x in pos {
			d += *x * *x;
		}
		if d.is_zero() {
			continue;
		}
		let f = (*mass + T::one()) * layout.settings.kg / d;
		for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
			*speed -= f * *pos;
		}
	}
}

pub fn apply_gravity_sg<T: Coord + std::fmt::Debug, const N: usize>(layout: &mut Layout<T, N>) {
	for ((mass, pos), speed) in layout
		.masses
		.iter()
		.zip(layout.points.iter())
		.zip(layout.speeds.iter_mut())
	{
		let f = (*mass + T::one()) * layout.settings.kg;
		for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
			*speed -= f * *pos;
		}
	}
}
