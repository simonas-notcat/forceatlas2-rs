use crate::{layout::Layout, util::*};

use std::marker::PhantomData;

pub struct Node<'a, T: Coord> {
	pub mass: &'a T,
	pub n2_iter: NodeIter2<'a, T>,
	pub pos: &'a [T],
	pub speed: &'a mut [T],
}

pub struct NodeIter<'a, T: Coord> {
	pub(crate) layout: SendPtr<Layout<T>>,
	pub offset: usize,
	pub(crate) _phantom: PhantomData<&'a mut Layout<T>>,
}

pub struct Node2<'a, T: Coord> {
	pub mass: &'a T,
	pub pos: &'a [T],
	pub speed: &'a mut [T],
}

pub struct NodeIter2<'a, T: Coord> {
	pub(crate) layout: SendPtr<Layout<T>>,
	pub offset: usize,
	pub(crate) _phantom: PhantomData<&'a mut Layout<T>>,
}

impl<'a, T: Coord> Iterator for NodeIter<'a, T> {
	type Item = Node<'a, T>;

	fn next(&mut self) -> Option<Self::Item> {
		let layout = unsafe { self.layout.0.as_mut() };
		if let Some(mass) = layout.masses.get(self.offset) {
			Some({
				let next_offset = self.offset + layout.settings.dimensions;
				let ret = Node {
					mass,
					n2_iter: NodeIter2 {
						layout: self.layout,
						offset: next_offset,
						_phantom: PhantomData::default(),
					},
					pos: unsafe { layout.points.points.get_unchecked(self.offset..next_offset) },
					speed: unsafe {
						self.layout
							.0
							.as_mut()
							.speeds
							.points
							.get_unchecked_mut(self.offset..next_offset)
					},
				};
				self.offset = next_offset;
				ret
			})
		} else {
			None
		}
	}
}

impl<'a, T: Coord> Iterator for NodeIter2<'a, T> {
	type Item = Node2<'a, T>;

	fn next(&mut self) -> Option<Self::Item> {
		let layout = unsafe { self.layout.0.as_mut() };
		if let Some(mass) = layout.masses.get(self.offset) {
			Some({
				let next_offset = self.offset + layout.settings.dimensions;
				let ret = Node2 {
					mass,
					pos: unsafe { layout.points.points.get_unchecked(self.offset..next_offset) },
					speed: unsafe {
						self.layout
							.0
							.as_mut()
							.speeds
							.points
							.get_unchecked_mut(self.offset..next_offset)
					},
				};
				self.offset = next_offset;
				ret
			})
		} else {
			None
		}
	}
}

// ------------------

pub struct NodePar<'a, T: Coord> {
	pub ind: usize,
	pub mass: &'a T,
	pub n2_iter: NodeParIter2<'a, T>,
	pub pos: &'a [T],
	pub speed: &'a mut [T],
}

pub struct NodeParIter<'a, T: Coord> {
	pub end: usize,
	pub(crate) layout: SendPtr<Layout<T>>,
	pub n2_start: usize,
	pub n2_end: usize,
	pub offset: usize,
	pub(crate) _phantom: PhantomData<&'a mut Layout<T>>,
}

pub struct NodePar2<'a, T: Coord> {
	pub ind: usize,
	pub mass: &'a T,
	pub pos: &'a [T],
	pub speed: &'a mut [T],
}

pub struct NodeParIter2<'a, T: Coord> {
	pub end: usize,
	pub(crate) layout: SendPtr<Layout<T>>,
	pub offset: usize,
	pub(crate) _phantom: PhantomData<&'a mut Layout<T>>,
}

impl<'a, T: Coord> Iterator for NodeParIter<'a, T> {
	type Item = NodePar<'a, T>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.offset < self.end {
			Some({
				let layout = unsafe { self.layout.0.as_mut() };
				let next_offset = self.offset + layout.settings.dimensions;
				let ret = NodePar {
					ind: self.offset / layout.settings.dimensions,
					mass: unsafe { layout.masses.get_unchecked(self.offset) },
					n2_iter: NodeParIter2 {
						end: self.n2_end,
						layout: self.layout,
						offset: self.n2_start.max(next_offset),
						_phantom: PhantomData::default(),
					},
					pos: unsafe { layout.points.points.get_unchecked(self.offset..next_offset) },
					speed: unsafe {
						self.layout
							.0
							.as_mut()
							.speeds
							.points
							.get_unchecked_mut(self.offset..next_offset)
					},
				};
				self.offset = next_offset;
				ret
			})
		} else {
			None
		}
	}
}

impl<'a, T: Coord> Iterator for NodeParIter2<'a, T> {
	type Item = NodePar2<'a, T>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.offset < self.end {
			Some({
				let layout = unsafe { self.layout.0.as_mut() };
				let next_offset = self.offset + layout.settings.dimensions;
				let ret = NodePar2 {
					ind: self.offset / layout.settings.dimensions,
					mass: unsafe { layout.masses.get_unchecked(self.offset) },
					pos: unsafe { layout.points.points.get_unchecked(self.offset..next_offset) },
					speed: unsafe {
						self.layout
							.0
							.as_mut()
							.speeds
							.points
							.get_unchecked_mut(self.offset..next_offset)
					},
				};
				self.offset = next_offset;
				ret
			})
		} else {
			None
		}
	}
}
