use crate::T;

use forceatlas2::*;
use gio::prelude::*;
use gtk::prelude::*;
use rand::Rng;
use std::{
	rc::Rc,
	sync::{Arc, RwLock},
	thread,
	time::Duration,
};

const STANDBY_SLEEP: Duration = Duration::from_millis(50);
const DRAW_SLEEP: Duration = Duration::from_millis(30);
const INVALID_BG_COLOR: gdk::RGBA = gdk::RGBA {
	red: 1.0,
	green: 0.0,
	blue: 0.0,
	alpha: 0.1,
};

enum MsgToGtk {
	Resize,
	Update,
}

struct MsgFromGtk {
	redraw: bool,
	resize: bool,
}

struct Pixbuf(gdk_pixbuf::Pixbuf);

unsafe impl Send for Pixbuf {}
unsafe impl Sync for Pixbuf {}

fn build_ui(
	app: &gtk::Application,
	rx: Arc<RwLock<Option<glib::Receiver<MsgToGtk>>>>,
	tx: Arc<RwLock<MsgFromGtk>>,
	compute: Arc<RwLock<bool>>,
	layout: Arc<RwLock<Layout<T>>>,
	settings: Arc<RwLock<Settings<T>>>,
	pixbuf: Arc<RwLock<Option<Pixbuf>>>,
	draw_edges: Arc<RwLock<bool>>,
	edge_color: Arc<RwLock<(u8, u8, u8)>>,
	draw_nodes: Arc<RwLock<bool>>,
	node_color: Arc<RwLock<(u8, u8, u8)>>,
	zoom: Arc<RwLock<T>>,
	nb_iters: Arc<RwLock<usize>>,
) {
	let glade_src = include_str!("gui.glade");
	let builder = gtk::Builder::new();
	builder.add_from_string(glade_src).unwrap();

	let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
	window.set_application(Some(app));

	let graph_area: gtk::Image = builder.get_object("graph_area").unwrap();
	let graph_box: gtk::ScrolledWindow = builder.get_object("graph_box").unwrap();
	let compute_button: gtk::ToggleButton = builder.get_object("bt_compute").unwrap();
	let reset_button: gtk::Button = builder.get_object("bt_reset").unwrap();
	let save_img_button: gtk::Button = builder.get_object("bt_save_img").unwrap();
	let chunk_size_input: gtk::Entry = builder.get_object("chunk_size").unwrap();
	let ka_input: gtk::Entry = builder.get_object("ka").unwrap();
	let kg_input: gtk::Entry = builder.get_object("kg").unwrap();
	let kr_input: gtk::Entry = builder.get_object("kr").unwrap();
	let draw_edges_input: gtk::CheckButton = builder.get_object("draw_edges").unwrap();
	let edge_color_input: gtk::ColorButton = builder.get_object("edge_color").unwrap();
	let draw_nodes_input: gtk::CheckButton = builder.get_object("draw_nodes").unwrap();
	let node_color_input: gtk::ColorButton = builder.get_object("node_color").unwrap();
	let zoom_input: gtk::Entry = builder.get_object("zoom").unwrap();
	let nb_iters_disp: gtk::Label = builder.get_object("nb_iters").unwrap();

	let save_img_window: gtk::FileChooserDialog = builder.get_object("save_img_window").unwrap();
	let siw_cancel_button: gtk::Button = builder.get_object("siw_bt_cancel").unwrap();
	let siw_save_button: gtk::Button = builder.get_object("siw_bt_save").unwrap();
	let siw_filename: gtk::Entry = builder.get_object("siw_filename").unwrap();
	let siw_filetype: gtk::ComboBox = builder.get_object("siw_filetype").unwrap();

	{
		let settings = settings.read().unwrap();
		chunk_size_input.set_text(&settings.chunk_size.unwrap_or(0).to_string());
		ka_input.set_text(&settings.ka.to_string());
		kg_input.set_text(&settings.kg.to_string());
		kr_input.set_text(&settings.kr.to_string());
	}
	draw_edges_input.set_active(*draw_edges.read().unwrap());
	edge_color_input.set_rgba({
		let edge_color = edge_color.read().unwrap();
		&gdk::RGBA {
			red: edge_color.0 as f64 / 255.,
			green: edge_color.1 as f64 / 255.,
			blue: edge_color.2 as f64 / 255.,
			alpha: 1.,
		}
	});
	draw_nodes_input.set_active(*draw_nodes.read().unwrap());
	node_color_input.set_rgba({
		let node_color = node_color.read().unwrap();
		&gdk::RGBA {
			red: node_color.0 as f64 / 255.,
			green: node_color.1 as f64 / 255.,
			blue: node_color.2 as f64 / 255.,
			alpha: 1.,
		}
	});
	zoom_input.set_text(&zoom.read().unwrap().to_string());

	{
		let layout = layout.read().unwrap();
		let nb_nodes_disp: gtk::Label = builder.get_object("nb_nodes").unwrap();
		let nb_edges_disp: gtk::Label = builder.get_object("nb_edges").unwrap();
		nb_nodes_disp.set_text(&layout.masses.len().to_string());
		nb_edges_disp.set_text(&layout.edges.len().to_string());
	}

	graph_area.connect_key_press_event({
		let zoom_input = zoom_input.clone();
		let zoom = zoom.clone();
		move |_, event| {
			if let Some(key) = event.get_keyval().name() {
				match key.as_str() {
					"KP_Add" | "plus" => {
						let zoom = *zoom.read().unwrap();
						zoom_input.set_text(&(zoom * 1.1).to_string());
					}
					"KP_Subtract" | "minus" => {
						let zoom = *zoom.read().unwrap();
						zoom_input.set_text(&(zoom * (1. / 1.1)).to_string())
					}
					"KP_0" | "0" => zoom_input.set_text("1"),
					_ => {}
				}
			}
			Inhibit(false)
		}
	});

	let graph_area = Rc::new(graph_area);
	let edge_color_input = Rc::new(edge_color_input);
	let save_img_window = Rc::new(save_img_window);

	compute_button.connect_toggled({
		let tx = tx.clone();
		move |bt| {
			if let Ok(mut compute) = compute.write() {
				if *compute {
					tx.write().unwrap().redraw = true;
				}
				*compute = bt.get_active();
			}
		}
	});

	reset_button.connect_clicked({
		let layout = layout.clone();
		let tx = tx.clone();
		let nb_iters = nb_iters.clone();
		move |_| {
			let mut rng = rand::thread_rng();
			let mut layout = layout.write().unwrap();
			layout.old_speeds.points.fill(0.0);
			layout.points.points.fill_with(|| rng.gen_range(-1.0..1.0));
			tx.write().unwrap().redraw = true;
			*nb_iters.write().unwrap() = 0;
		}
	});

	save_img_button.connect_clicked({
		let save_img_window = save_img_window.clone();
		move |_| {
			save_img_window.show_all();
		}
	});

	save_img_window
		.connect_delete_event(move |save_img_window, _| save_img_window.hide_on_delete());

	siw_cancel_button.connect_clicked({
		let save_img_window = save_img_window.clone();
		move |_| save_img_window.hide()
	});

	siw_save_button.connect_clicked({
		let pixbuf = pixbuf.clone();
		move |_| {
			let filename = siw_filename.get_text();
			let filename = filename.as_str();
			let filetype = siw_filetype.get_active_id();
			let filetype = filetype.as_ref().map_or_else(
				|| {
					if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
						"jpeg"
					} else if filename.ends_with(".tiff") {
						"tiff"
					} else if filename.ends_with(".bmp") {
						"bmp"
					} else {
						"png"
					}
				},
				|filetype| filetype.as_str(),
			);
			if let Some(pixbuf) = pixbuf.read().unwrap().as_ref() {
				let path = save_img_window.get_current_folder().unwrap().join(filename);
				if let Err(e) = pixbuf.0.savev(path, filetype, &[]) {
					eprintln!("Error while saving: {:?}", e);
				}
				save_img_window.hide()
			}
		}
	});

	chunk_size_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(chunk_size) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				if let Ok(mut settings) = settings.write() {
					settings.chunk_size = if chunk_size == 0 {
						None
					} else {
						Some(chunk_size)
					};
					if let Ok(mut layout) = layout.write() {
						layout.set_settings(settings.clone());
					}
				}
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	ka_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(ka) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				if let Ok(mut settings) = settings.write() {
					settings.ka = ka;
					if let Ok(mut layout) = layout.write() {
						layout.set_settings(settings.clone());
					}
				}
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	kg_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(kg) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				if let Ok(mut settings) = settings.write() {
					settings.kg = kg;
					if let Ok(mut layout) = layout.write() {
						layout.set_settings(settings.clone());
					}
				}
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	kr_input.connect_changed({
		move |entry| {
			if let Ok(kr) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				if let Ok(mut settings) = settings.write() {
					settings.kr = kr;
					if let Ok(mut layout) = layout.write() {
						layout.set_settings(settings.clone());
					}
				}
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	draw_edges_input.connect_toggled({
		let tx = tx.clone();
		move |draw_edges_input| {
			*draw_edges.write().unwrap() = draw_edges_input.get_active();
			tx.write().unwrap().redraw = true;
		}
	});

	edge_color_input.connect_color_set({
		let tx = tx.clone();
		move |edge_color_input| {
			let c = edge_color_input.get_rgba();
			*edge_color.write().unwrap() = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
			);
			tx.write().unwrap().redraw = true;
		}
	});

	draw_nodes_input.connect_toggled({
		let tx = tx.clone();
		move |draw_nodes_input| {
			*draw_nodes.write().unwrap() = draw_nodes_input.get_active();
			tx.write().unwrap().redraw = true;
		}
	});

	node_color_input.connect_color_set({
		let tx = tx.clone();
		move |node_color_input| {
			let c = node_color_input.get_rgba();
			*node_color.write().unwrap() = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
			);
			tx.write().unwrap().redraw = true;
		}
	});

	zoom_input.connect_changed({
		let pixbuf = pixbuf.clone();
		let graph_box = graph_box.clone();
		let tx = tx.clone();
		let zoom = zoom.clone();
		move |entry| {
			if let Ok(val) = entry.get_text().parse() {
				if val > 0.0 {
					entry.override_background_color(gtk::StateFlags::NORMAL, None);
					let mut zoom = zoom.write().unwrap();
					let mut pixbuf = pixbuf.write().unwrap();
					*zoom = val;
					*pixbuf = gdk_pixbuf::Pixbuf::new(
						gdk_pixbuf::Colorspace::Rgb,
						false,
						8,
						(graph_box.get_allocated_width() as T * *zoom) as i32,
						(graph_box.get_allocated_height() as T * *zoom) as i32,
					)
					.map(Pixbuf);
					tx.write().unwrap().redraw = true;
					return;
				}
			}
			entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
		}
	});

	let resize_handler = {
		let pixbuf = pixbuf.clone();
		let tx = tx.clone();
		move || {
			let mut pixbuf = pixbuf.write().unwrap();
			tx.write().unwrap().redraw = true;
			let zoom = zoom.read().unwrap();
			*pixbuf = gdk_pixbuf::Pixbuf::new(
				gdk_pixbuf::Colorspace::Rgb,
				false,
				8,
				(graph_box.get_allocated_width() as T * *zoom) as i32,
				(graph_box.get_allocated_height() as T * *zoom) as i32,
			)
			.map(Pixbuf);
		}
	};

	window.connect_configure_event({
		move |_, _| {
			tx.write().unwrap().resize = true;
			true
		}
	});

	if let Some(rx) = rx.write().unwrap().take() {
		rx.attach(None, move |msg| {
			match msg {
				MsgToGtk::Update => {
					if let Some(pixbuf) = pixbuf.read().unwrap().as_ref() {
						graph_area.set_from_pixbuf(Some(&pixbuf.0));
					}
					nb_iters_disp.set_text(&nb_iters.read().unwrap().to_string());
				}
				MsgToGtk::Resize => resize_handler(),
			}
			glib::Continue(true)
		});
	}

	window.show_all();
}

pub fn run(
	compute: Arc<RwLock<bool>>,
	layout: Arc<RwLock<Layout<T>>>,
	settings: Arc<RwLock<Settings<T>>>,
	nb_iters: Arc<RwLock<usize>>,
) {
	let application = gtk::Application::new(
		Some("org.framagit.ZettaScript.forceatlas2.examples.viz"),
		Default::default(),
	)
	.unwrap();

	let (tx, rx) = glib::MainContext::sync_channel(glib::PRIORITY_DEFAULT, 4);
	let rx = Arc::new(RwLock::new(Some(rx)));
	let msg_from_gtk = Arc::new(RwLock::new(MsgFromGtk {
		redraw: true,
		resize: false,
	}));
	let pixbuf = Arc::new(RwLock::new(None));
	let draw_edges = Arc::new(RwLock::new(true));
	let edge_color = Arc::new(RwLock::new((5, 5, 5)));
	let draw_nodes = Arc::new(RwLock::new(true));
	let node_color = Arc::new(RwLock::new((255, 0, 0)));
	let zoom = Arc::new(RwLock::new(1.0));

	application.connect_activate({
		let compute = compute.clone();
		let layout = layout.clone();
		let pixbuf = pixbuf.clone();
		let draw_edges = draw_edges.clone();
		let edge_color = edge_color.clone();
		let draw_nodes = draw_nodes.clone();
		let node_color = node_color.clone();
		let msg_from_gtk = msg_from_gtk.clone();
		move |app| {
			build_ui(
				app,
				rx.clone(),
				msg_from_gtk.clone(),
				compute.clone(),
				layout.clone(),
				settings.clone(),
				pixbuf.clone(),
				draw_edges.clone(),
				edge_color.clone(),
				draw_nodes.clone(),
				node_color.clone(),
				zoom.clone(),
				nb_iters.clone(),
			)
		}
	});

	thread::spawn(move || loop {
		thread::sleep(if *compute.read().unwrap() {
			if let Some(pixbuf) = pixbuf.write().unwrap().as_ref() {
				let layout = layout.read().unwrap();
				crate::drawer::draw_graph(
					layout,
					(pixbuf.0.get_width(), pixbuf.0.get_height()),
					unsafe { pixbuf.0.get_pixels() },
					pixbuf.0.get_rowstride(),
					*draw_edges.read().unwrap(),
					*edge_color.read().unwrap(),
					*draw_nodes.read().unwrap(),
					*node_color.read().unwrap(),
				);
				tx.send(MsgToGtk::Update).unwrap();
			}
			let mut msg_from_gtk = msg_from_gtk.write().unwrap();
			if msg_from_gtk.resize {
				msg_from_gtk.resize = false;
				msg_from_gtk.redraw = false;
				tx.send(MsgToGtk::Resize).unwrap();
			}
			DRAW_SLEEP
		} else {
			let mut msg_from_gtk = msg_from_gtk.write().unwrap();
			if msg_from_gtk.resize {
				msg_from_gtk.resize = false;
				msg_from_gtk.redraw = false;
				tx.send(MsgToGtk::Resize).unwrap();
			} else if msg_from_gtk.redraw {
				msg_from_gtk.redraw = false;
				if let Some(pixbuf) = pixbuf.write().unwrap().as_ref() {
					let layout = layout.read().unwrap();
					crate::drawer::draw_graph(
						layout,
						(pixbuf.0.get_width(), pixbuf.0.get_height()),
						unsafe { pixbuf.0.get_pixels() },
						pixbuf.0.get_rowstride(),
						*draw_edges.read().unwrap(),
						*edge_color.read().unwrap(),
						*draw_nodes.read().unwrap(),
						*node_color.read().unwrap(),
					);
					tx.send(MsgToGtk::Update).unwrap();
				}
			}
			STANDBY_SLEEP
		});
	});

	application.run(&[]);
}
