use crate::{
	layout::*,
	trees::{Body, Vec2, Vec3, VecN},
	util::*,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub(crate) struct NodeBodyN<T, const N: usize> {
	pos: VecN<T, N>,
	mass: T,
}

type NodeBody2<T> = NodeBodyN<T, 2>;
type NodeBody3<T> = NodeBodyN<T, 3>;

impl<T, const N: usize> Body<T, N> for NodeBodyN<T, N>
where
	T: Coord,
{
	fn mass(&self) -> T {
		self.mass
	}

	fn pos(&self) -> VecN<T, N> {
		self.pos
	}

	fn add_mass(&mut self, mass: T) {
		self.mass += mass
	}
}

pub fn apply_repulsion_2d<T: Coord + Send + Sync>(layout: &mut Layout<T, 2>) {
	let mut points_iter = layout.points.iter();
	let Some(point) = points_iter.next() else {
		return;
	};
	let (mut min_x, mut min_y, mut max_x, mut max_y) = (point[0], point[1], point[0], point[1]);
	for point in points_iter {
		if point[0] < min_x {
			min_x = point[0];
		} else if point[0] > max_x {
			max_x = point[0];
		}
		if point[1] < min_y {
			min_y = point[1];
		} else if point[1] > max_y {
			max_y = point[1];
		}
	}

	let mut bump = layout.bump.lock();
	let mut tree =
		crate::trees::Tree::<crate::trees::Node2<T, NodeBody2<T>>, T, NodeBody2<T>, 2>::from_bump(
			&mut bump,
		);
	let mut root = tree.new_root((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)));

	for node in layout.points.iter().zip(layout.masses.iter()) {
		root.add_body(NodeBody2 {
			pos: Vec2::new(node.0[0], node.0[1]),
			mass: *node.1 + T::one(),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.barnes_hut;

	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((particle, speed), mass)| {
			let f = root.apply(
				Vec2::new(particle[0], particle[1]),
				theta,
				(),
				&|bb, b, mm, d, _| (bb - b) * mm / d,
			) * kr * (*mass + T::one());
			speed[0] -= f.x();
			speed[1] -= f.y();
		});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_3d<T: Coord + Send + Sync>(layout: &mut Layout<T, 3>) {
	let mut points_iter = layout.points.iter();
	let Some(point) = points_iter.next() else {
		return;
	};
	let (mut min_x, mut min_y, mut min_z, mut max_x, mut max_y, mut max_z) =
		(point[0], point[1], point[2], point[0], point[1], point[2]);
	for point in points_iter {
		if point[0] < min_x {
			min_x = point[0];
		} else if point[0] > max_x {
			max_x = point[0];
		}
		if point[1] < min_y {
			min_y = point[1];
		} else if point[1] > max_y {
			max_y = point[1];
		}
		if point[2] < min_z {
			min_z = point[2];
		} else if point[2] > max_z {
			max_z = point[2];
		}
	}

	let mut bump = layout.bump.lock();
	let mut tree =
		crate::trees::Tree::<crate::trees::Node3<T, NodeBody3<T>>, T, NodeBody3<T>, 3>::from_bump(
			&mut bump,
		);
	let mut root = tree.new_root((
		Vec3::new(min_x, min_y, min_z),
		Vec3::new(max_x, max_y, max_z),
	));

	for node in layout.points.iter().zip(layout.masses.iter()) {
		root.add_body(NodeBody3 {
			pos: Vec3::new(node.0[0], node.0[1], node.0[2]),
			mass: *node.1 + T::one(),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.barnes_hut;

	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((particle, speed), mass)| {
			let f = root.apply(
				Vec3::new(particle[0], particle[1], particle[2]),
				theta,
				(),
				&|bb, b, mm, d, _| (bb - b) * mm / d,
			) * kr * (*mass + T::one());
			speed[0] -= f.x();
			speed[1] -= f.y();
			speed[2] -= f.z();
		});
	std::mem::drop(root);
	tree.clear();
}
