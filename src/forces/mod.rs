pub mod attraction;
pub mod gravity;
pub mod repulsion;

use crate::{
	layout::{Layout, Settings},
	util::*,
};

#[doc(hidden)]
pub trait Attraction<T: Coord + std::fmt::Debug, const N: usize> {
	fn choose_attraction(settings: &Settings<T>) -> fn(&mut Layout<T, N>);
}

#[doc(hidden)]
pub trait Repulsion<T: Coord + std::fmt::Debug, const N: usize> {
	fn choose_repulsion(settings: &Settings<T>) -> fn(&mut Layout<T, N>);
}

impl<T, const N: usize> Attraction<T, N> for Layout<T, N>
where
	T: Coord + std::fmt::Debug,
{
	#[allow(clippy::collapsible_else_if)]
	fn choose_attraction(settings: &Settings<T>) -> fn(&mut Layout<T, N>) {
		if settings.prevent_overlapping.is_some() {
			if settings.lin_log {
				if settings.dissuade_hubs {
					attraction::apply_attraction_dh_log_po
				} else {
					attraction::apply_attraction_log_po
				}
			} else {
				if settings.dissuade_hubs {
					attraction::apply_attraction_dh_po
				} else {
					attraction::apply_attraction_po
				}
			}
		} else {
			if settings.lin_log {
				if settings.dissuade_hubs {
					attraction::apply_attraction_dh_log
				} else {
					attraction::apply_attraction_log
				}
			} else {
				if settings.dissuade_hubs {
					attraction::apply_attraction_dh
				} else {
					attraction::apply_attraction
				}
			}
		}
	}
}

pub fn choose_gravity<T: Coord + std::fmt::Debug, const N: usize>(
	settings: &Settings<T>,
) -> fn(&mut Layout<T, N>) {
	if settings.kg.is_zero() {
		return |_| {};
	}
	if settings.strong_gravity {
		gravity::apply_gravity_sg
	} else {
		gravity::apply_gravity
	}
}

default impl<T: Send + Sync> Repulsion<T, 2> for Layout<T, 2>
where
	T: Coord + std::fmt::Debug,
{
	fn choose_repulsion(settings: &Settings<T>) -> fn(&mut Layout<T, 2>) {
		if settings.prevent_overlapping.is_some() {
			todo!()
		} else {
			repulsion::apply_repulsion_2d
		}
	}
}

default impl<T: Send + Sync> Repulsion<T, 3> for Layout<T, 3>
where
	T: Coord + std::fmt::Debug,
{
	fn choose_repulsion(settings: &Settings<T>) -> fn(&mut Layout<T, 3>) {
		if settings.prevent_overlapping.is_some() {
			todo!()
		} else {
			repulsion::apply_repulsion_3d
		}
	}
}
