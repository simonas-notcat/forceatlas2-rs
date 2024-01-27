use crate::{layout::Layout, util::*};

use itertools::izip;

pub fn apply_gravity<T: Coord + std::fmt::Debug>(layout: &mut Layout<T>) {
	for (mass, pos, speed) in izip!(
		layout.masses.iter(),
		layout.points.iter(),
		layout.speeds.iter_mut()
	) {
		let d = norm(pos);
		if d.is_zero() {
			continue;
		}
		let f = (*mass + T::one()) * layout.settings.kg / d;
		for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
			*speed -= f * *pos;
		}
	}
}

pub fn apply_gravity_sg<T: Coord + std::fmt::Debug>(layout: &mut Layout<T>) {
	for (mass, pos, speed) in izip!(
		layout.masses.iter(),
		layout.points.iter(),
		layout.speeds.iter_mut()
	) {
		let f = (*mass + T::one()) * layout.settings.kg;
		for (speed, pos) in speed.iter_mut().zip(pos.iter()) {
			*speed -= f * *pos;
		}
	}
}
