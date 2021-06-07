mod drawer;
mod gui;

use forceatlas2::*;
use std::{
	io::BufRead,
	sync::{Arc, RwLock},
	thread,
	time::Duration,
};

const STANDBY_SLEEP: Duration = Duration::from_millis(50);
const COMPUTE_SLEEP: Duration = Duration::from_millis(1);
type T = f32;

fn main() {
	let file = std::fs::File::open(std::env::args().nth(1).expect("Usage: viz <csv_file>"))
		.expect("Cannot open file");

	let mut nodes = 0usize;
	let mut edges = Vec::<(usize, usize)>::new();
	for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
		let line = line.expect("Error reading CSV");
		let mut columns = line.split(&[' ', '\t', ',', ';'][..]);
		if let (Some(n1), Some(n2)) = (columns.next(), columns.find(|&c| !c.is_empty())) {
			if let (Ok(n1), Ok(n2)) = (n1.parse(), n2.parse()) {
				if n1 > nodes {
					nodes = n1;
				}
				if n2 > nodes {
					nodes = n2;
				}
				if n1 != n2 {
					edges.push(if n1 < n2 { (n1, n2) } else { (n2, n1) });
				}
			} else {
				eprintln!("Ignored line {} has bad number format", i);
			}
		} else {
			eprintln!("Ignored line {} has <2 columns", i);
		}
	}
	nodes += 1;

	println!("Nodes: {}", nodes);

	let settings = Settings {
		chunk_size: Some(256),
		dimensions: 2,
		dissuade_hubs: false,
		ka: 1.0,
		kg: 1.0,
		kr: 1.0,
		lin_log: false,
		prevent_overlapping: None,
		strong_gravity: false,
	};

	let layout = Arc::new(RwLock::new(Layout::<T>::from_graph(
		edges,
		Nodes::Degree(nodes),
		settings.clone(),
	)));

	let compute = Arc::new(RwLock::new(false));
	let settings = Arc::new(RwLock::new(settings));

	thread::spawn({
		let compute = compute.clone();
		let layout = layout.clone();
		move || loop {
			thread::sleep(if *compute.read().unwrap() {
				layout.write().unwrap().iteration();
				COMPUTE_SLEEP
			} else {
				STANDBY_SLEEP
			});
		}
	});

	gui::run(compute, layout, settings);
}
