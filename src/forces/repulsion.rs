use crate::{
	layout::*,
	trees::{Body, Vec2, Vec3, VecN},
	util::*,
};

use rayon::prelude::*;

pub(crate) struct NodeBodyN<T, C, const N: usize> {
	pos: VecN<T, N>,
	mass: T,
	custom: C,
}

type NodeBody2<T, C> = NodeBodyN<T, C, 2>;
type NodeBody3<T, C> = NodeBodyN<T, C, 3>;

impl<T: Coord, C: Copy, const N: usize> Body<T, C, N> for NodeBodyN<T, C, N> {
	fn mass(&self) -> T {
		self.mass
	}

	fn pos(&self) -> VecN<T, N> {
		self.pos
	}

	fn add_mass(&mut self, mass: T) {
		self.mass += mass
	}

	fn custom(&self) -> C {
		self.custom
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
	let mut tree = crate::trees::Tree::<
		crate::trees::Node2<T, NodeBody2<T, ()>>,
		T,
		(),
		NodeBody2<T, ()>,
		2,
	>::from_bump(&mut bump);
	let mut root = tree.new_root((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)));

	for (pos, mass) in layout.points.iter().zip(layout.masses.iter()) {
		root.add_body(NodeBody2 {
			pos: VecN(*pos),
			mass: *mass + T::one(),
			custom: (),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.theta;

	let f1 = |bb, b, mm, d, _| (bb - b) * mm / d;
	let f2 = |bb, b, mm, d, _, _| (bb - b) * mm / d;
	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((particle, speed), mass)| {
			let f = root.apply(VecN(*particle), theta, (), &f1, &f2) * kr * (*mass + T::one());
			speed[0] -= f.x();
			speed[1] -= f.y();
		});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_2d_po<T: Coord + Send + Sync>(layout: &mut Layout<T, 2>) {
	let mut kr_prime = layout.settings.prevent_overlapping.unwrap();
	let sizes = layout.sizes.as_ref().unwrap();

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
	let mut tree = crate::trees::Tree::<
		crate::trees::Node2<T, NodeBody2<T, T>>,
		T,
		T,
		NodeBody2<T, T>,
		2,
	>::from_bump(&mut bump);
	let mut root = tree.new_root((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)));

	for ((pos, mass), size) in layout
		.points
		.iter()
		.zip(layout.masses.iter())
		.zip(sizes.iter())
	{
		root.add_body(NodeBody2 {
			pos: VecN(*pos),
			mass: *mass + T::one(),
			custom: *size,
		});
	}

	let kr = layout.settings.kr;
	kr_prime /= kr;
	let theta = layout.settings.theta;

	let f1 = |bb, b, mm, d, _| (bb - b) * mm / d;
	let f2 = |bb, b, mm, d, s1, s2| {
		let d_prime: T = d - s1 - s2;
		if d_prime.is_positive() {
			(bb - b) * mm / d_prime
		} else {
			(bb - b) * mm * kr_prime
		}
	};
	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.zip(sizes.par_iter())
		.for_each(|(((particle, speed), mass), size)| {
			let f = root.apply(VecN(*particle), theta, *size, &f1, &f2) * kr * (*mass + T::one());
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
	let mut tree = crate::trees::Tree::<
		crate::trees::Node3<T, NodeBody3<T, ()>>,
		T,
		(),
		NodeBody3<T, ()>,
		3,
	>::from_bump(&mut bump);
	let mut root = tree.new_root((
		Vec3::new(min_x, min_y, min_z),
		Vec3::new(max_x, max_y, max_z),
	));

	for (pos, mass) in layout.points.iter().zip(layout.masses.iter()) {
		root.add_body(NodeBody3 {
			pos: VecN(*pos),
			mass: *mass + T::one(),
			custom: (),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.theta;

	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.for_each(|((particle, speed), mass)| {
			let f = root.apply(
				VecN(*particle),
				theta,
				(),
				&|bb, b, mm, d, _| (bb - b) * mm / d,
				&|bb, b, mm, d, _, _| (bb - b) * mm / d,
			) * kr * (*mass + T::one());
			speed[0] -= f.x();
			speed[1] -= f.y();
			speed[2] -= f.z();
		});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_3d_po<T: Coord + Send + Sync>(layout: &mut Layout<T, 3>) {
	let mut kr_prime = layout.settings.prevent_overlapping.unwrap();
	let sizes = layout.sizes.as_ref().unwrap();

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
	let mut tree = crate::trees::Tree::<
		crate::trees::Node3<T, NodeBody3<T, T>>,
		T,
		T,
		NodeBody3<T, T>,
		3,
	>::from_bump(&mut bump);
	let mut root = tree.new_root((
		Vec3::new(min_x, min_y, min_z),
		Vec3::new(max_x, max_y, max_z),
	));

	for ((pos, mass), size) in layout.points.iter().zip(layout.masses.iter()).zip(sizes) {
		root.add_body(NodeBody3 {
			pos: VecN(*pos),
			mass: *mass + T::one(),
			custom: *size,
		});
	}

	let kr = layout.settings.kr;
	kr_prime /= kr;
	let theta = layout.settings.theta;

	let f1 = |bb, b, mm, d, _| (bb - b) * mm / d;
	let f2 = |bb, b, mm, d, s1, s2| {
		let d_prime: T = d - s1 - s2;
		if d_prime.is_positive() {
			(bb - b) * mm / d_prime
		} else {
			(bb - b) * mm * kr_prime
		}
	};
	layout
		.points
		.par_iter()
		.zip(layout.speeds.par_iter_mut())
		.zip(layout.masses.par_iter())
		.zip(sizes.par_iter())
		.for_each(|(((particle, speed), mass), size)| {
			let f = root.apply(VecN(*particle), theta, *size, &f1, &f2) * kr * (*mass + T::one());
			speed[0] -= f.x();
			speed[1] -= f.y();
			speed[2] -= f.z();
		});
	std::mem::drop(root);
	tree.clear();
}
