use forceatlas2::*;
use std::io::BufRead;

const DEFAULT_ITERATIONS: usize = 1000;

fn main() {
	let mut args = std::env::args().skip(1);
	let filename = args.next().expect(&format!(
		"Usage: csv_import <csv_file> [iterations (default={})]",
		DEFAULT_ITERATIONS
	));
	let iterations = args.next().map_or(DEFAULT_ITERATIONS, |arg| {
		arg.parse().expect("Invalid argument iterations")
	});
	let file = std::fs::File::open(filename).expect("Cannot open file");

	let mut nodes = 0usize;
	let mut edges = Vec::<((usize, usize), f32)>::new();
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
				if n1 != n2 {
					edges.push((if n1 < n2 { (n1, n2) } else { (n2, n1) }, 1.0));
				}
			} else {
				eprintln!("Ignored line {} has bad number format", i);
			}
		} else {
			eprintln!("Ignored line {} has <2 columns", i);
		}
	}
	nodes += 1;

	eprintln!("Nodes: {}", nodes);

	let mut layout = Layout::<f32, 2>::from_graph_with_degree_mass(
		edges,
		(0..nodes).map(|_| 1.0),
		Settings {
			theta: 0.5,
			ka: 0.01,
			kg: 0.001,
			kr: 0.002,
			lin_log: false,
			speed: 1.0,
			prevent_overlapping: None,
			strong_gravity: false,
		},
	);

	for i in 0..iterations {
		eprint!("{}/{}\r", i, iterations);
		layout.iteration();
	}

	for node in layout.nodes {
		println!("{}\t{}", node.pos[0], node.pos[1]);
	}
}
