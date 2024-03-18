use crate::{layout::Layout, util::*};

use rayon::prelude::*;

pub fn apply_gravity<T: Coord + Send + Sync, const N: usize>(layout: &mut Layout<T, N>) {
	layout.nodes.par_iter_mut().for_each(|node| {
		let mut d = T::zero();
		for x in node.pos {
			d += x * x;
		}
		if d.is_zero() {
			return;
		}
		node.speed -= node.pos * (node.mass + T::one()) * layout.settings.kg / d;
	})
}

pub fn apply_gravity_sg<T: Coord + Send + Sync, const N: usize>(layout: &mut Layout<T, N>) {
	layout.nodes.par_iter_mut().for_each(|node| {
		node.speed -= node.pos * (node.mass + T::one()) * layout.settings.kg;
	})
}
