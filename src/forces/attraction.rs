use crate::{layout::Layout, util::*};

// TODO weighted impl
pub fn apply_attraction<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			let f = (n2_pos[i] - n1_pos[i]) * layout.settings.ka;
			n1_speed[i] += f;
			n2_speed[i] -= f;
		}
	}
}

pub fn apply_attraction_dh<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		let n1_mass = layout.masses[*n1];
		for i in 0usize..N {
			let f = (n2_pos[i] - n1_pos[i]) * layout.settings.ka / n1_mass;
			n1_speed[i] += f;
			n2_speed[i] -= f;
		}
	}
}

pub fn apply_attraction_log<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		if d.is_zero() {
			continue;
		}
		d = d.sqrt();
		let f = d.ln_1p() / d * layout.settings.ka;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}

pub fn apply_attraction_dh_log<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		if d.is_zero() {
			continue;
		}
		d = d.sqrt();
		let n1_mass = layout.masses[*n1];
		let f = d.ln_1p() / d * layout.settings.ka / n1_mass;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}

pub fn apply_attraction_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	let sizes = layout.sizes.as_ref().unwrap();
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0usize..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		d = d.sqrt();
		let dprime = d - sizes[*n1] - sizes[*n2];
		if !dprime.is_positive() {
			continue;
		}
		let f = dprime / d * layout.settings.ka;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}

pub fn apply_attraction_dh_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	let sizes = layout.sizes.as_ref().unwrap();
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0usize..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		d = d.sqrt();
		let dprime = d - sizes[*n1] - sizes[*n2];
		if !dprime.is_positive() {
			continue;
		}
		let n1_mass = layout.masses[*n1];
		let f = dprime / d * layout.settings.ka / n1_mass;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}

pub fn apply_attraction_log_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	let sizes = layout.sizes.as_ref().unwrap();
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0usize..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		d = d.sqrt();
		let dprime = d - sizes[*n1] - sizes[*n2];
		if !dprime.is_positive() {
			continue;
		}
		// TODO check formula
		let f = dprime.ln_1p() / dprime * layout.settings.ka;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}

pub fn apply_attraction_dh_log_po<T: Coord, const N: usize>(layout: &mut Layout<T, N>) {
	let sizes = layout.sizes.as_ref().unwrap();
	for (n1, n2) in layout.edges.iter() {
		let (n1_pos, n2_pos) = get_2_mut(&mut layout.points, *n1, *n2);
		let mut d = T::zero();
		let mut dv = [T::zero(); N];
		for i in 0usize..N {
			dv[i] = n2_pos[i] - n1_pos[i];
			d += dv[i] * dv[i];
		}
		d = d.sqrt();
		let dprime = d - sizes[*n1] - sizes[*n2];
		if !dprime.is_positive() {
			continue;
		}
		// TODO check formula
		let n1_mass = layout.masses[*n1];
		let f = dprime.ln_1p() / dprime * layout.settings.ka / n1_mass;
		let (n1_speed, n2_speed) = get_2_mut(&mut layout.speeds, *n1, *n2);
		for i in 0..N {
			n1_speed[i] += f * dv[i];
			n2_speed[i] -= f * dv[i];
		}
	}
}
