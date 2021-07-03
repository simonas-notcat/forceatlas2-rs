use crate::T;

use forceatlas2::*;
use nalgebra::{Matrix2x3, Unit, Vector3};

type Rgb = (u8, u8, u8);

pub struct RgbGradient {
	start_color: Rgb,
	start_value: T,
	end_color: Rgb,
	end_value: T,
}

pub enum NodeColor {
	Fixed(Rgb),
	Mass(RgbGradient),
}

// https://www.codeguru.com/cpp/cpp/algorithms/general/article.php/c15989/Tip-An-Optimized-Formula-for-Alpha-Blending-Pixels.htm
pub fn blend(s: u8, d: u8, a: u8) -> u8 {
	(((s as u16 * a as u16) + (d as u16 * (255 - a) as u16)) >> 8) as u8
}

// https://github.com/deep110/ada/blob/master/src/shape/line2d.rs
fn draw_line(
	buffer: &mut [u8],
	size: (i32, i32),
	rowstride: i32,
	color: (u8, u8, u8, u8),
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
				buffer[offset] = blend(color.0, buffer[offset], color.3);
				buffer[offset + 1] = blend(color.1, buffer[offset], color.3);
				buffer[offset + 2] = blend(color.2, buffer[offset], color.3);
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
				buffer[offset] = blend(color.0, buffer[offset], color.3);
				buffer[offset + 1] = blend(color.1, buffer[offset], color.3);
				buffer[offset + 2] = blend(color.2, buffer[offset], color.3);
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
	edge_color: (u8, u8, u8, u8),
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

pub fn _draw_graph_3d(
	layout: std::sync::RwLockReadGuard<Layout<T>>,
	size: (i32, i32),
	pixels: &mut [u8],
	rowstride: i32,
	draw_edges: bool,
	edge_color: (u8, u8, u8, u8),
) {
	let camera = cam_geom::Camera::new(
		cam_geom::IntrinsicParametersPerspective::from(cam_geom::PerspectiveParams {
			fx: 100.0,
			fy: 100.0,
			skew: 0.0,
			cx: size.0 as f32 / 2.0,
			cy: size.1 as f32 / 2.0,
		}),
		cam_geom::ExtrinsicParameters::from_view(
			&Vector3::new(10.0, 0.0, 0.0),
			&Vector3::new(0.0, 0.0, 0.0),
			&Unit::new_normalize(Vector3::new(0.0, 0.0, 1.0)),
		),
	);

	pixels.fill(255);

	if draw_edges {
		for (h1, h2) in layout.edges.iter() {
			let p1 = layout.points.get(*h1);
			let p2 = layout.points.get(*h2);
			let proj = camera.world_to_pixel(&cam_geom::Points::new(unsafe {
				Matrix2x3::new(
					*p1.get_unchecked(0),
					*p1.get_unchecked(1),
					*p1.get_unchecked(2),
					*p2.get_unchecked(0),
					*p2.get_unchecked(1),
					*p2.get_unchecked(2),
				)
			}));
			draw_line(
				pixels,
				size,
				rowstride,
				edge_color,
				unsafe {
					(
						proj.data.row(0)[0].to_int_unchecked(),
						proj.data.row(0)[1].to_int_unchecked(),
					)
				},
				unsafe {
					(
						proj.data.row(1)[0].to_int_unchecked(),
						proj.data.row(1)[1].to_int_unchecked(),
					)
				},
			);
		}
	}
}
