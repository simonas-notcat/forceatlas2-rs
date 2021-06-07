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

enum Msg {
	Update,
}

struct Pixbuf(gdk_pixbuf::Pixbuf);

unsafe impl Send for Pixbuf {}
unsafe impl Sync for Pixbuf {}

fn build_ui(
	app: &gtk::Application,
	rx: Arc<RwLock<Option<glib::Receiver<Msg>>>>,
	compute: Arc<RwLock<bool>>,
	layout: Arc<RwLock<Layout<T>>>,
	settings: Arc<RwLock<Settings<T>>>,
	pixbuf: Arc<RwLock<Option<Pixbuf>>>,
	redraw: Arc<RwLock<bool>>,
	edge_color: Arc<RwLock<(u8, u8, u8)>>,
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
	let chunk_size_input: gtk::Entry = builder.get_object("chunk_size").unwrap();
	let ka_input: gtk::Entry = builder.get_object("ka").unwrap();
	let kg_input: gtk::Entry = builder.get_object("kg").unwrap();
	let kr_input: gtk::Entry = builder.get_object("kr").unwrap();
	let edge_color_input: gtk::ColorButton = builder.get_object("edge_color").unwrap();

	{
		let settings = settings.read().unwrap();
		chunk_size_input.set_text(&settings.chunk_size.unwrap_or(0).to_string());
		ka_input.set_text(&settings.ka.to_string());
		kg_input.set_text(&settings.kg.to_string());
		kr_input.set_text(&settings.kr.to_string());
	}

	let graph_area = Rc::new(graph_area);
	let edge_color_input = Rc::new(edge_color_input);

	compute_button.connect_toggled({
		move |bt| {
			if let Ok(mut compute) = compute.write() {
				*compute = bt.get_active();
			}
		}
	});

	reset_button.connect_clicked({
		let layout = layout.clone();
		let redraw = redraw.clone();
		move |_| {
			let mut rng = rand::thread_rng();
			let mut layout = layout.write().unwrap();
			layout.old_speeds.points.fill(0.0);
			layout.points.points.fill_with(|| rng.gen_range(-1.0..1.0));
			*redraw.write().unwrap() = true;
		}
	});

	chunk_size_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(chunk_size) = entry.get_buffer().get_text().parse() {
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
			if let Ok(ka) = entry.get_buffer().get_text().parse() {
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
			if let Ok(kg) = entry.get_buffer().get_text().parse() {
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
			if let Ok(kr) = entry.get_buffer().get_text().parse() {
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

	edge_color_input.clone().connect_color_set({
		let redraw = redraw.clone();
		move |_| {
			let c = edge_color_input.get_rgba();
			*edge_color.write().unwrap() = ((c.red*255.) as u8, (c.green*255.) as u8, (c.blue*255.) as u8);
			*redraw.write().unwrap() = true;
		}
	});

	let resize_handler = {
		let pixbuf = pixbuf.clone();
		move || {
			let mut pixbuf = pixbuf.write().unwrap();
			*redraw.write().unwrap() = true;
			*pixbuf = gdk_pixbuf::Pixbuf::new(
				gdk_pixbuf::Colorspace::Rgb,
				false,
				8,
				graph_box.get_allocated_width(),
				graph_box.get_allocated_height(),
			)
			.map(Pixbuf);
		}
	};

	window.connect_configure_event({
		let resize_handler = resize_handler.clone();
		move |_, _| {
			resize_handler();
			true
		}
	});

	if let Ok(mut rx) = rx.write() {
		if let Some(rx) = rx.take() {
			rx.attach(None, move |_| {
				if let Some(pixbuf) = pixbuf.read().unwrap().as_ref() {
					graph_area.set_from_pixbuf(Some(&pixbuf.0));
				}
				glib::Continue(true)
			});
		}
	}

	window.show_all();
}

pub fn run(
	compute: Arc<RwLock<bool>>,
	layout: Arc<RwLock<Layout<T>>>,
	settings: Arc<RwLock<Settings<T>>>,
) {
	let application = gtk::Application::new(
		Some("org.framagit.ZettaScript.forceatlas2.examples.viz"),
		Default::default(),
	)
	.unwrap();

	let (tx, rx) = glib::MainContext::sync_channel(glib::PRIORITY_DEFAULT, 1);
	let rx = Arc::new(RwLock::new(Some(rx)));
	let pixbuf = Arc::new(RwLock::new(None));
	let redraw = Arc::new(RwLock::new(true));
	let edge_color = Arc::new(RwLock::new((5, 5, 5)));

	application.connect_activate({
		let compute = compute.clone();
		let layout = layout.clone();
		let pixbuf = pixbuf.clone();
		let redraw = redraw.clone();
		let edge_color = edge_color.clone();
		move |app| {
			build_ui(
				app,
				rx.clone(),
				compute.clone(),
				layout.clone(),
				settings.clone(),
				pixbuf.clone(),
				redraw.clone(),
				edge_color.clone(),
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
					*edge_color.read().unwrap(),
				);
				tx.send(Msg::Update).unwrap();
			}
			DRAW_SLEEP
		} else {
			let mut redraw = redraw.write().unwrap();
			if *redraw {
				*redraw = false;
				if let Some(pixbuf) = pixbuf.write().unwrap().as_ref() {
					let layout = layout.read().unwrap();
					crate::drawer::draw_graph(
						layout,
						(pixbuf.0.get_width(), pixbuf.0.get_height()),
						unsafe { pixbuf.0.get_pixels() },
						pixbuf.0.get_rowstride(),
						*edge_color.read().unwrap(),
					);
					tx.send(Msg::Update).unwrap();
				}
			}
			STANDBY_SLEEP
		});
	});

	application.run(&[]);
}
