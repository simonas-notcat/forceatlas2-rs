use crate::{layout::Layout, util::*};

// TODO weighted impl
pub fn apply_attraction<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		for ((n1_speed, n2_speed), (n1_pos, n2_pos)) in n1
			.speed
			.iter_mut()
			.zip(n2.speed.iter_mut())
			.zip(n1.pos.iter().zip(n2.pos.iter()))
		{
			let f = (*n2_pos - *n1_pos) * layout.settings.ka;
			*n1_speed += f;
			*n2_speed -= f;
		}
	}
}

pub fn apply_attraction_log<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		if d.is_zero() {
			continue;
		}
		d = d.sqrt();
		let f = d.ln_1p() / d * layout.settings.ka;
		for ((n1_speed, n2_speed), dvi) in
			n1.speed.iter_mut().zip(n2.speed.iter_mut()).zip(dv.iter())
		{
			*n1_speed += f * *dvi;
			*n2_speed -= f * *dvi;
		}
	}
}

pub fn apply_attraction_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		d = d.sqrt();
		let dprime = d - n1.size - n2.size;
		if !dprime.is_positive() {
			continue;
		}
		let f = dprime / d * layout.settings.ka;
		for ((n1_speed, n2_speed), dvi) in
			n1.speed.iter_mut().zip(n2.speed.iter_mut()).zip(dv.iter())
		{
			*n1_speed += f * *dvi;
			*n2_speed -= f * *dvi;
		}
	}
}

pub fn apply_attraction_log_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1, n2) = get_2_mut(&mut layout.nodes, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for ((n1_pos, n2_pos), dvi) in n1.pos.iter().zip(n2.pos.iter()).zip(dv.iter_mut()) {
			*dvi = *n2_pos - *n1_pos;
			d += *dvi * *dvi;
		}
		d = d.sqrt();
		let dprime = d - n1.size - n2.size;
		if !dprime.is_positive() {
			continue;
		}
		// TODO check formula
		let f = dprime.ln_1p() / dprime * layout.settings.ka;
		for ((n1_speed, n2_speed), dvi) in
			n1.speed.iter_mut().zip(n2.speed.iter_mut()).zip(dv.iter())
		{
			*n1_speed += f * *dvi;
			*n2_speed -= f * *dvi;
		}
	}
}
