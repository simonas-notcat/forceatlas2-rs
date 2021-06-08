use crate::T;

use forceatlas2::*;

// https://github.com/deep110/ada/blob/master/src/shape/line2d.rs
fn draw_line(
	buffer: &mut [u8],
	size: (i32, i32),
	rowstride: i32,
	color: (u8, u8, u8),
	mut p1: (i32, i32),
	mut p2: (i32, i32),
) {
	let mut steep = false;

	if (p1.0 - p2.0).abs() < (p1.1 - p2.1).abs() {
		std::mem::swap(&mut p1.0, &mut p1.1);
		std::mem::swap(&mut p2.0, &mut p2.1);
		steep = true;
	}
	if p1.0 > p2.0 {
		std::mem::swap(&mut p1, &mut p2);
	}
	let dx = p2.0 - p1.0;
	let derror = ((p2.1 - p1.1) * 2).abs();
	let mut error = 0;
	let mut y = p1.1;
	if steep {
		for x in p1.0..(p2.0 + 1) {
			if y >= 0 && y < size.0 && x >= 0 && x < size.1 {
				let offset = (x * rowstride + y * 3) as usize;
				buffer[offset] = buffer[offset].saturating_sub(color.0);
				buffer[offset + 1] = buffer[offset + 1].saturating_sub(color.1);
				buffer[offset + 2] = buffer[offset + 2].saturating_sub(color.2);
			}

			error += derror;
			if error > dx {
				y += if p2.1 > p1.1 { 1 } else { -1 };
				error -= dx * 2;
			}
		}
	} else {
		for x in p1.0..(p2.0 + 1) {
			if x >= 0 && x < size.0 && y >= 0 && y < size.1 {
				let offset = (y * rowstride + x * 3) as usize;
				buffer[offset] = buffer[offset].saturating_sub(color.0);
				buffer[offset + 1] = buffer[offset + 1].saturating_sub(color.1);
				buffer[offset + 2] = buffer[offset + 2].saturating_sub(color.2);
			}

			error += derror;
			if error > dx {
				y += if p2.1 > p1.1 { 1 } else { -1 };
				error -= dx * 2;
			}
		}
	}
}

pub fn draw_disk(
	buffer: &mut [u8],
	size: (i32, i32),
	rowstride: i32,
	color: (u8, u8, u8),
	center: (i32, i32),
	radius: i32,
) {
	let r2 = radius.pow(2);
	for y in 0..radius {
		let mx = ((r2 - y * y) as f64).sqrt() as i32;
		for x in 0..mx {
			if center.0 + x >= 0
				&& center.0 + x < size.0
				&& center.1 + y >= 0
				&& center.1 + y < size.1
			{
				let offset = ((center.1 + y) * rowstride + (center.0 + x) * 3) as usize;
				buffer[offset] = color.0;
				buffer[offset + 1] = color.1;
				buffer[offset + 2] = color.2;
			}
			if center.0 - x >= 0
				&& center.0 - x < size.0
				&& center.1 + y >= 0
				&& center.1 + y < size.1
			{
				let offset = ((center.1 + y) * rowstride + (center.0 - x) * 3) as usize;
				buffer[offset] = color.0;
				buffer[offset + 1] = color.1;
				buffer[offset + 2] = color.2;
			}
			if center.0 + x >= 0
				&& center.0 + x < size.0
				&& center.1 - y >= 0
				&& center.1 - y < size.1
			{
				let offset = ((center.1 - y) * rowstride + (center.0 + x) * 3) as usize;
				buffer[offset] = color.0;
				buffer[offset + 1] = color.1;
				buffer[offset + 2] = color.2;
			}
			if center.0 - x >= 0
				&& center.0 - x < size.0
				&& center.1 - y >= 0
				&& center.1 - y < size.1
			{
				let offset = ((center.1 - y) * rowstride + (center.0 - x) * 3) as usize;
				buffer[offset] = color.0;
				buffer[offset + 1] = color.1;
				buffer[offset + 2] = color.2;
			}
		}
	}
}

pub fn draw_graph(
	layout: std::sync::RwLockReadGuard<Layout<T>>,
	size: (i32, i32),
	pixels: &mut [u8],
	rowstride: i32,
	draw_edges: bool,
	edge_color: (u8, u8, u8),
	draw_nodes: bool,
	node_color: (u8, u8, u8),
) {
	pixels.fill(255);

	let mut min_v = layout.points.get_clone(0);
	let mut max_v = min_v.clone();
	let min = min_v.as_mut_slice();
	let max = max_v.as_mut_slice();
	for pos in layout.points.iter() {
		if pos[0] < min[0] {
			min[0] = pos[0];
		}
		if pos[1] < min[1] {
			min[1] = pos[1];
		}
		if pos[0] > max[0] {
			max[0] = pos[0];
		}
		if pos[1] > max[1] {
			max[1] = pos[1];
		}
	}
	let graph_size = (max[0] - min[0], max[1] - min[1]);
	let factor = {
		let factors = (size.0 as T / graph_size.0, size.1 as T / graph_size.1);
		if factors.0 > factors.1 {
			min[0] -= (size.0 as T / factors.1 - graph_size.0) / 2.0;
			factors.1
		} else {
			min[1] -= (size.1 as T / factors.0 - graph_size.1) / 2.0;
			factors.0
		}
	};

	if draw_edges {
		for (h1, h2) in layout.edges.iter() {
			draw_line(
				pixels,
				size,
				rowstride,
				edge_color,
				{
					let pos = layout.points.get(*h1);
					unsafe {
						(
							((pos[0] - min[0]) * factor).to_int_unchecked::<i32>(),
							((pos[1] - min[1]) * factor).to_int_unchecked::<i32>(),
						)
					}
				},
				{
					let pos = layout.points.get(*h2);
					unsafe {
						(
							((pos[0] - min[0]) * factor).to_int_unchecked::<i32>(),
							((pos[1] - min[1]) * factor).to_int_unchecked::<i32>(),
						)
					}
				},
			);
		}
	}

	if draw_nodes {
		for pos in layout.points.iter() {
			draw_disk(
				pixels,
				size,
				rowstride,
				node_color,
				{
					unsafe {
						(
							((pos[0] - min[0]) * factor).to_int_unchecked::<i32>(),
							((pos[1] - min[1]) * factor).to_int_unchecked::<i32>(),
						)
					}
				},
				2,
			);
		}
	}
}
