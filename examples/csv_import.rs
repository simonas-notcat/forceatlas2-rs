use forceatlas2::*;
use plotters::prelude::*;
use std::io::BufRead;

const SIZE: (u32, u32) = (1024, 1024);

const ITERATIONS: u32 = 500;
const ANIM_MODE: bool = true;
const DRAW_EDGES: bool = true;

fn main() {
	let file = std::fs::File::open(
		std::env::args()
			.skip(1)
			.next()
			.expect("Usage: csv_import <csv_file>"),
	)
	.expect("Cannot open file");

	let mut nodes = 0usize;
	let mut edges = Vec::<(usize, usize)>::new();
	for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
		let line = line.expect("Error reading CSV");
		let mut columns = line.split(&[' ', '\t', ',', ';'][..]);
		if let (Some(n1), Some(n2)) = (columns.next(), columns.skip_while(|&c| c.is_empty()).next())
		{
			if let (Ok(n1), Ok(n2)) = (n1.parse(), n2.parse()) {
				if n1 > nodes {
					nodes = n1;
				}
				if n2 > nodes {
					nodes = n2;
				}
				edges.push((n1, n2));
			} else {
				eprintln!("Ignored line {} has bad number format", i);
			}
		} else {
			eprintln!("Ignored line {} has <2 columns", i);
		}
	}

	let mut layout = Layout::<f64>::from_graph(
		edges,
		nodes + 1,
		Settings {
			dimensions: 2,
			dissuade_hubs: false,
			ka: 100.0,
			kg: 0.1,
			kr: 0.00001,
			scaling_ratio: 0.005,
			lin_log: false,
			prevent_overlapping: None,
			strong_gravity: false,
			barnes_hut: None,
		},
	);

	eprintln!("Computing layout...");
	for i in 0..ITERATIONS {
		if ANIM_MODE {
			draw_graph(&layout, i);
		}
		println!("{}/{}", i, ITERATIONS);
		layout.iteration();
	}
	draw_graph(&layout, ITERATIONS);
}

fn draw_graph(layout: &Layout<f64>, iteration: u32) {
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
		let factors = (
			f64::from(SIZE.0) / graph_size.0,
			f64::from(SIZE.1) / graph_size.1,
		);
		if factors.0 > factors.1 {
			min[0] -= (f64::from(SIZE.0) / factors.1 - graph_size.0) / 2.0;
			factors.1
		} else {
			min[1] -= (f64::from(SIZE.1) / factors.0 - graph_size.1) / 2.0;
			factors.0
		}
	};
	// println!("size:  {:?}", graph_size);
	// println!("scale: {}", factor);

	let path = if ANIM_MODE {
		format!("target/graph-{}.png", iteration)
	} else {
		"target/graph.png".into()
	};
	let root = BitMapBackend::new(&path, SIZE).into_drawing_area();
	root.fill(&WHITE).unwrap();

	// draw edges
	if DRAW_EDGES {
		for (h1, h2) in layout.edges.iter() {
			root.draw(&PathElement::new(
				vec![
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
				],
				Into::<ShapeStyle>::into(&RGBColor(5, 5, 5).mix(0.05)).filled(),
			))
			.unwrap();
		}
	}

	// draw nodes
	for pos in layout.points.iter() {
		root.draw(&Circle::new(
			unsafe {
				(
					((pos[0] - min[0]) * factor).to_int_unchecked::<i32>(),
					((pos[1] - min[1]) * factor).to_int_unchecked::<i32>(),
				)
			},
			2,
			Into::<ShapeStyle>::into(&RED).filled(),
		))
		.unwrap();
	}

	// draw text
	root.draw(&Text::new(
		format!(
			"Parameters: ka: {} kg: {} kr: {} Smooth: true, Iteration: {}, scale: {}",
			layout.settings.ka,
			layout.settings.kg,
			layout.settings.kr,
			iteration,
			512.0 / factor // relative to layout size
		),
		(5, 5),
		("sans-serif", 24.0).into_font(),
	))
	.unwrap();
}
