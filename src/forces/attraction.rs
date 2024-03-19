use crate::{layout::Layout, util::*};

use num_traits::Zero;

pub fn apply_attraction<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for ((n1, n2), weight) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let f = (n2.pos - n1.pos) * layout.settings.ka * *weight;
		n1.speed += f;
		n2.speed -= f;
	}
}

pub fn apply_attraction_log<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for ((n1, n2), weight) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = VecN::<T, N>::zero();
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		if d.is_zero() {
			continue;
		}
		d = d.sqrt();
		let f = dv * (d.ln_1p() / d * layout.settings.ka * *weight);
		n1.speed += f;
		n2.speed -= f;
	}
}

pub fn apply_attraction_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for ((n1, n2), weight) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = VecN::<T, N>::zero();
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		d = d.sqrt();
		let dprime = d - n1.size - n2.size;
		if !dprime.is_positive() {
			continue;
		}
		let f = dv * (dprime / d * layout.settings.ka * *weight);
		n1.speed += f;
		n2.speed -= f;
	}
}

pub fn apply_attraction_log_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for ((n1, n2), weight) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = VecN::<T, N>::zero();
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		d = d.sqrt();
		let dprime = d - n1.size - n2.size;
		if !dprime.is_positive() {
			continue;
		}
		let f = dv * (dprime.ln_1p() / d * layout.settings.ka * *weight);
		n1.speed += f;
		n2.speed -= f;
	}
}
