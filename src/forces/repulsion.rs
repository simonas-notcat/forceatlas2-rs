use crate::{
	layout::*,
	trees::{Body, VecN},
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
	let mut nodes_iter = layout.nodes.iter();
	let Some(first_node) = nodes_iter.next() else {
		return;
	};
	let (mut min_pos, mut max_pos) = (first_node.pos, first_node.pos);
	for Node { pos, .. } in nodes_iter {
		for ((min, max), val) in min_pos.iter_mut().zip(max_pos.iter_mut()).zip(pos.iter()) {
			if val < min {
				*min = *val;
			} else if val > max {
				*max = *val;
			}
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
	let mut root = tree.new_root((VecN(min_pos), VecN(max_pos)));

	for node in layout.nodes.iter() {
		root.add_body(NodeBody2 {
			pos: VecN(node.pos),
			mass: node.mass + T::one(),
			custom: (),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.theta;

	let f1 = |bb, b, mm, d, _| (bb - b) * mm / d;
	let f2 = |bb, b, mm, d, _, _| (bb - b) * mm / d;
	layout.nodes.par_iter_mut().for_each(|node| {
		let f = root.apply(VecN(node.pos), theta, (), &f1, &f2) * kr * (node.mass + T::one());
		node.speed[0] -= f.x();
		node.speed[1] -= f.y();
	});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_2d_po<T: Coord + Send + Sync>(layout: &mut Layout<T, 2>) {
	let mut kr_prime = layout.settings.prevent_overlapping.unwrap();

	let mut nodes_iter = layout.nodes.iter();
	let Some(first_node) = nodes_iter.next() else {
		return;
	};
	let (mut min_pos, mut max_pos) = (first_node.pos, first_node.pos);
	for Node { pos, .. } in nodes_iter {
		for ((min, max), val) in min_pos.iter_mut().zip(max_pos.iter_mut()).zip(pos.iter()) {
			if val < min {
				*min = *val;
			} else if val > max {
				*max = *val;
			}
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
	let mut root = tree.new_root((VecN(min_pos), VecN(max_pos)));

	for node in layout.nodes.iter() {
		root.add_body(NodeBody2 {
			pos: VecN(node.pos),
			mass: node.mass + T::one(),
			custom: node.size,
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
	layout.nodes.par_iter_mut().for_each(|node| {
		let f =
			root.apply(VecN(node.pos), theta, node.size, &f1, &f2) * kr * (node.mass + T::one());
		node.speed[0] -= f.x();
		node.speed[1] -= f.y();
	});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_3d<T: Coord + Send + Sync>(layout: &mut Layout<T, 3>) {
	let mut nodes_iter = layout.nodes.iter();
	let Some(first_node) = nodes_iter.next() else {
		return;
	};
	let (mut min_pos, mut max_pos) = (first_node.pos, first_node.pos);
	for Node { pos, .. } in nodes_iter {
		for ((min, max), val) in min_pos.iter_mut().zip(max_pos.iter_mut()).zip(pos.iter()) {
			if val < min {
				*min = *val;
			} else if val > max {
				*max = *val;
			}
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
	let mut root = tree.new_root((VecN(min_pos), VecN(max_pos)));

	for node in layout.nodes.iter() {
		root.add_body(NodeBody3 {
			pos: VecN(node.pos),
			mass: node.mass + T::one(),
			custom: (),
		});
	}

	let kr = layout.settings.kr;
	let theta = layout.settings.theta;

	layout.nodes.par_iter_mut().for_each(|node| {
		let f = root.apply(
			VecN(node.pos),
			theta,
			(),
			&|bb, b, mm, d, _| (bb - b) * mm / d,
			&|bb, b, mm, d, _, _| (bb - b) * mm / d,
		) * kr * (node.mass + T::one());
		node.speed[0] -= f.x();
		node.speed[1] -= f.y();
		node.speed[2] -= f.z();
	});
	std::mem::drop(root);
	tree.clear();
}

pub fn apply_repulsion_3d_po<T: Coord + Send + Sync>(layout: &mut Layout<T, 3>) {
	let mut kr_prime = layout.settings.prevent_overlapping.unwrap();

	let mut nodes_iter = layout.nodes.iter();
	let Some(first_node) = nodes_iter.next() else {
		return;
	};
	let (mut min_pos, mut max_pos) = (first_node.pos, first_node.pos);
	for Node { pos, .. } in nodes_iter {
		for ((min, max), val) in min_pos.iter_mut().zip(max_pos.iter_mut()).zip(pos.iter()) {
			if val < min {
				*min = *val;
			} else if val > max {
				*max = *val;
			}
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
	let mut root = tree.new_root((VecN(min_pos), VecN(max_pos)));

	for node in layout.nodes.iter() {
		root.add_body(NodeBody3 {
			pos: VecN(node.pos),
			mass: node.mass + T::one(),
			custom: node.size,
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
	layout.nodes.par_iter_mut().for_each(|node| {
		let f =
			root.apply(VecN(node.pos), theta, node.size, &f1, &f2) * kr * (node.mass + T::one());
		node.speed[0] -= f.x();
		node.speed[1] -= f.y();
		node.speed[2] -= f.z();
	});
	std::mem::drop(root);
	tree.clear();
}
