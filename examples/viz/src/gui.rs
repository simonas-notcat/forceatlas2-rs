use crate::T;

use forceatlas2::*;
use gio::prelude::*;
use gtk::{prelude::*, SettingsExt};
use parking_lot::RwLock;
use rand::Rng;
use static_rc::StaticRc;
use std::{rc::Rc, sync::Arc, thread, time::Duration};

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
	edge_color: Arc<RwLock<(u8, u8, u8, u8)>>,
	draw_nodes: Arc<RwLock<bool>>,
	node_color: Arc<RwLock<(u8, u8, u8)>>,
	node_radius: Arc<RwLock<i32>>,
	bg_color: Arc<RwLock<(u8, u8, u8)>>,
	zoom: Arc<RwLock<T>>,
	nb_iters: Arc<RwLock<usize>>,
) {
	let builder = gtk::Builder::new();
	builder.add_from_string(include_str!("gui.glade")).unwrap();

	let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
	window.set_application(Some(app));

	let graph_area: gtk::Image = builder.get_object("graph_area").unwrap();
	let graph_box: gtk::ScrolledWindow = builder.get_object("graph_box").unwrap();
	let graph_viewport: gtk::Viewport = builder.get_object("graph_viewport").unwrap();
	let graph_hadj: gtk::Adjustment = graph_box.get_hadjustment().unwrap();
	let graph_vadj: gtk::Adjustment = graph_box.get_vadjustment().unwrap();
	let compute_button: gtk::ToggleButton = builder.get_object("bt_compute").unwrap();
	let reset_button: gtk::Button = builder.get_object("bt_reset").unwrap();
	let save_img_button: gtk::Button = builder.get_object("bt_save_img").unwrap();
	let copy_img_button: gtk::Button = builder.get_object("bt_copy_img").unwrap();
	let chunk_size_input: gtk::Entry = builder.get_object("chunk_size").unwrap();
	let ka_input: gtk::Entry = builder.get_object("ka").unwrap();
	let kg_input: gtk::Entry = builder.get_object("kg").unwrap();
	let kr_input: gtk::Entry = builder.get_object("kr").unwrap();
	let speed_input: gtk::Entry = builder.get_object("speed").unwrap();
	let draw_edges_input: gtk::CheckButton = builder.get_object("draw_edges").unwrap();
	let edge_color_input: gtk::ColorButton = builder.get_object("edge_color").unwrap();
	let draw_nodes_input: gtk::CheckButton = builder.get_object("draw_nodes").unwrap();
	let node_color_input: gtk::ColorButton = builder.get_object("node_color").unwrap();
	let node_radius_input: gtk::Entry = builder.get_object("node_radius").unwrap();
	let bg_color_input: gtk::ColorButton = builder.get_object("bg_color").unwrap();
	let zoom_input: gtk::Entry = builder.get_object("zoom").unwrap();
	let nb_iters_disp: gtk::Label = builder.get_object("nb_iters").unwrap();

	let save_img_window: gtk::FileChooserDialog = builder.get_object("save_img_window").unwrap();
	let siw_cancel_button: gtk::Button = builder.get_object("siw_bt_cancel").unwrap();
	let siw_save_button: gtk::Button = builder.get_object("siw_bt_save").unwrap();
	let siw_filename: gtk::Entry = builder.get_object("siw_filename").unwrap();
	let siw_filetype: gtk::ComboBox = builder.get_object("siw_filetype").unwrap();

	if window.get_settings().map_or(false, |s| {
		s.get_property_gtk_theme_name()
			.map_or(false, |s| s.as_str().ends_with("-dark"))
	}) {
		let mut edge_color = edge_color.write();
		*edge_color = (255, 255, 255, 20);
		let mut node_color = node_color.write();
		*node_color = (255, 127, 0);
		let mut bg_color = bg_color.write();
		*bg_color = (0, 0, 0);
	}

	{
		let settings = settings.read();
		chunk_size_input.set_text(&settings.chunk_size.unwrap_or(0).to_string());
		ka_input.set_text(&settings.ka.to_string());
		kg_input.set_text(&settings.kg.to_string());
		kr_input.set_text(&settings.kr.to_string());
		speed_input.set_text(&settings.speed.to_string());
	}
	draw_edges_input.set_active(*draw_edges.read());
	edge_color_input.set_rgba({
		let edge_color = edge_color.read();
		&gdk::RGBA {
			red: edge_color.0 as f64 / 255.,
			green: edge_color.1 as f64 / 255.,
			blue: edge_color.2 as f64 / 255.,
			alpha: edge_color.3 as f64 / 255.,
		}
	});
	draw_nodes_input.set_active(*draw_nodes.read());
	node_color_input.set_rgba({
		let node_color = node_color.read();
		&gdk::RGBA {
			red: node_color.0 as f64 / 255.,
			green: node_color.1 as f64 / 255.,
			blue: node_color.2 as f64 / 255.,
			alpha: 1.,
		}
	});
	node_radius_input.set_text(&node_radius.read().to_string());
	bg_color_input.set_rgba({
		let bg_color = bg_color.read();
		&gdk::RGBA {
			red: bg_color.0 as f64 / 255.,
			green: bg_color.1 as f64 / 255.,
			blue: bg_color.2 as f64 / 255.,
			alpha: 1.,
		}
	});
	zoom_input.set_text(&zoom.read().to_string());

	{
		let layout = layout.read();
		let nb_nodes_disp: gtk::Label = builder.get_object("nb_nodes").unwrap();
		let nb_edges_disp: gtk::Label = builder.get_object("nb_edges").unwrap();
		nb_nodes_disp.set_text(&layout.masses.len().to_string());
		nb_edges_disp.set_text(&layout.edges.len().to_string());
	}

	let graph_area = StaticRc::<gtk::Image, 1, 1>::new(graph_area);
	let graph_adj =
		StaticRc::<(gtk::Adjustment, gtk::Adjustment), 1, 1>::new((graph_hadj, graph_vadj));
	let graph_drag = Rc::new(RwLock::new(None));
	let graph_gesture_drag = gtk::GestureDrag::new(&graph_viewport);
	graph_gesture_drag.set_touch_only(false);

	graph_gesture_drag.connect_drag_begin({
		let graph_area = graph_area.clone();
		let graph_drag = graph_drag.clone();
		let graph_adj = graph_adj.clone();
		move |_, _, _| {
			graph_area.grab_focus();
			*graph_drag.write() = Some((graph_adj.0.get_value(), graph_adj.1.get_value()));
		}
	});
	graph_gesture_drag.connect_drag_update({
		let graph_drag = graph_drag.clone();
		let graph_adj = graph_adj.clone();
		move |_, x, y| {
			if let Some((cx, cy)) = graph_drag.read().as_ref() {
				graph_adj.0.set_value(cx - x);
				graph_adj.1.set_value(cy - y);
			}
		}
	});
	graph_gesture_drag.connect_drag_end({
		move |_, x, y| {
			if let Some((cx, cy)) = graph_drag.write().take() {
				graph_adj.0.set_value(cx - x);
				graph_adj.1.set_value(cy - y);
			}
		}
	});

	graph_area.connect_key_press_event({
		let zoom_input = zoom_input.clone();
		let zoom = zoom.clone();
		move |_, event| {
			#[allow(clippy::no_effect, unused_must_use)]
			{
				&graph_gesture_drag; // avoid GC
			}

			if let Some(key) = event.get_keyval().name() {
				match key.as_str() {
					"KP_Add" | "plus" => {
						let zoom = *zoom.read();
						zoom_input.set_text(&(zoom * 1.1).to_string());
					}
					"KP_Subtract" | "minus" => {
						let zoom = *zoom.read();
						zoom_input.set_text(&(zoom * (1. / 1.1)).to_string())
					}
					"KP_0" | "0" => zoom_input.set_text("1"),
					_ => {}
				}
			}
			Inhibit(false)
		}
	});

	let edge_color_input = StaticRc::<gtk::ColorButton, 1, 1>::new(edge_color_input);
	let save_img_window = StaticRc::<gtk::FileChooserDialog, 1, 1>::new(save_img_window);
	let graph_viewport = StaticRc::<gtk::Viewport, 1, 1>::new(graph_viewport);

	compute_button.connect_toggled({
		let tx = tx.clone();
		move |bt| {
			let mut compute = compute.write();
			if *compute {
				tx.write().redraw = true;
			}
			*compute = bt.get_active();
		}
	});

	reset_button.connect_clicked({
		let layout = layout.clone();
		let tx = tx.clone();
		let nb_iters = nb_iters.clone();
		move |_| {
			let mut rng = rand::thread_rng();
			let mut layout = layout.write();
			layout.old_speeds.points.fill(0.0);
			layout.points.points.fill_with(|| rng.gen_range(-1.0..1.0));
			tx.write().redraw = true;
			*nb_iters.write() = 0;
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
			if let Some(pixbuf) = pixbuf.read().as_ref() {
				if let Some(current_folder) = save_img_window.get_current_folder() {
					if let Err(e) = pixbuf.0.savev(current_folder.join(filename), filetype, &[]) {
						eprintln!("Error while saving: {:?}", e);
					}
				} else {
					eprintln!("Cannot save: no current folder");
				}
				save_img_window.hide()
			}
		}
	});

	copy_img_button.connect_clicked({
		let pixbuf = pixbuf.clone();
		move |_| {
			if let Some(pixbuf) = pixbuf.read().as_ref() {
				gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_image(&pixbuf.0);
			}
		}
	});

	chunk_size_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(chunk_size) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				let mut settings = settings.write();
				settings.chunk_size = if chunk_size == 0 {
					None
				} else {
					Some(chunk_size)
				};
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
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
				let mut settings = settings.write();
				settings.ka = ka;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
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
				let mut settings = settings.write();
				settings.kg = kg;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	kr_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(kr) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				let mut settings = settings.write();
				settings.kr = kr;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	speed_input.connect_changed({
		move |entry| {
			if let Ok(speed) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				let mut settings = settings.write();
				settings.speed = speed;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	draw_edges_input.connect_toggled({
		let tx = tx.clone();
		move |draw_edges_input| {
			*draw_edges.write() = draw_edges_input.get_active();
			tx.write().redraw = true;
		}
	});

	edge_color_input.connect_color_set({
		let tx = tx.clone();
		move |edge_color_input| {
			let c = edge_color_input.get_rgba();
			*edge_color.write() = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
				(c.alpha * 255.) as u8,
			);
			tx.write().redraw = true;
		}
	});

	draw_nodes_input.connect_toggled({
		let tx = tx.clone();
		move |draw_nodes_input| {
			*draw_nodes.write() = draw_nodes_input.get_active();
			tx.write().redraw = true;
		}
	});

	node_color_input.connect_color_set({
		let tx = tx.clone();
		move |node_color_input| {
			let c = node_color_input.get_rgba();
			*node_color.write() = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
			);
			tx.write().redraw = true;
		}
	});

	node_radius_input.connect_changed({
		let tx = tx.clone();
		move |entry| {
			if let Ok(v) = entry.get_text().parse() {
				entry.override_background_color(gtk::StateFlags::NORMAL, None);
				let mut node_radius = node_radius.write();
				if *node_radius != v {
					tx.write().redraw = true;
				}
				*node_radius = v;
			} else {
				entry.override_background_color(gtk::StateFlags::NORMAL, Some(&INVALID_BG_COLOR));
			}
		}
	});

	bg_color_input.connect_color_set({
		let tx = tx.clone();
		move |bg_color_input| {
			let c = bg_color_input.get_rgba();
			*bg_color.write() = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
			);
			tx.write().redraw = true;
		}
	});

	zoom_input.connect_changed({
		let pixbuf = pixbuf.clone();
		let tx = tx.clone();
		let zoom = zoom.clone();
		let graph_viewport = graph_viewport.clone();
		move |entry| {
			if let Ok(val) = entry.get_text().parse() {
				if val > 0.0 {
					entry.override_background_color(gtk::StateFlags::NORMAL, None);
					let mut zoom = zoom.write();
					let mut pixbuf = pixbuf.write();
					*zoom = val;
					*pixbuf = gdk_pixbuf::Pixbuf::new(
						gdk_pixbuf::Colorspace::Rgb,
						false,
						8,
						(graph_viewport.get_allocated_width() as T * *zoom) as i32,
						(graph_viewport.get_allocated_height() as T * *zoom) as i32,
					)
					.map(Pixbuf);
					tx.write().redraw = true;
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
			let mut pixbuf = pixbuf.write();
			tx.write().redraw = true;
			let zoom = zoom.read();
			*pixbuf = gdk_pixbuf::Pixbuf::new(
				gdk_pixbuf::Colorspace::Rgb,
				false,
				8,
				(graph_viewport.get_allocated_width() as T * *zoom) as i32,
				(graph_viewport.get_allocated_height() as T * *zoom) as i32,
			)
			.map(Pixbuf);
		}
	};

	window.connect_configure_event({
		move |_, _| {
			tx.write().resize = true;
			true
		}
	});

	if let Some(rx) = rx.write().take() {
		rx.attach(None, move |msg| {
			match msg {
				MsgToGtk::Update => {
					if let Some(pixbuf) = pixbuf.read().as_ref() {
						graph_area.set_from_pixbuf(Some(&pixbuf.0));
					}
					nb_iters_disp.set_text(&nb_iters.read().to_string());
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
	let edge_color = Arc::new(RwLock::new((0, 0, 0, 20)));
	let draw_nodes = Arc::new(RwLock::new(true));
	let node_color = Arc::new(RwLock::new((255, 0, 0)));
	let node_radius = Arc::new(RwLock::new(2));
	let bg_color = Arc::new(RwLock::new((255, 255, 255)));
	let zoom = Arc::new(RwLock::new(1.0));

	application.connect_activate({
		let compute = compute.clone();
		let layout = layout.clone();
		let pixbuf = pixbuf.clone();
		let draw_edges = draw_edges.clone();
		let edge_color = edge_color.clone();
		let draw_nodes = draw_nodes.clone();
		let node_color = node_color.clone();
		let node_radius = node_radius.clone();
		let bg_color = bg_color.clone();
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
				node_radius.clone(),
				bg_color.clone(),
				zoom.clone(),
				nb_iters.clone(),
			)
		}
	});

	thread::spawn(move || loop {
		thread::sleep(if *compute.read() {
			if let Some(pixbuf) = pixbuf.write().as_ref() {
				let layout = layout.read();
				crate::drawer::draw_graph(
					layout,
					(pixbuf.0.get_width(), pixbuf.0.get_height()),
					unsafe { pixbuf.0.get_pixels() },
					pixbuf.0.get_rowstride(),
					*draw_edges.read(),
					*edge_color.read(),
					*draw_nodes.read(),
					*node_color.read(),
					*node_radius.read(),
					*bg_color.read(),
				);
				tx.send(MsgToGtk::Update).unwrap();
			}
			let mut msg_from_gtk = msg_from_gtk.write();
			if msg_from_gtk.resize {
				msg_from_gtk.resize = false;
				msg_from_gtk.redraw = false;
				tx.send(MsgToGtk::Resize).unwrap();
			}
			DRAW_SLEEP
		} else {
			let mut msg_from_gtk = msg_from_gtk.write();
			if msg_from_gtk.resize {
				msg_from_gtk.resize = false;
				msg_from_gtk.redraw = false;
				tx.send(MsgToGtk::Resize).unwrap();
			} else if msg_from_gtk.redraw {
				msg_from_gtk.redraw = false;
				if let Some(pixbuf) = pixbuf.write().as_ref() {
					let layout = layout.read();
					crate::drawer::draw_graph(
						layout,
						(pixbuf.0.get_width(), pixbuf.0.get_height()),
						unsafe { pixbuf.0.get_pixels() },
						pixbuf.0.get_rowstride(),
						*draw_edges.read(),
						*edge_color.read(),
						*draw_nodes.read(),
						*node_color.read(),
						*node_radius.read(),
						*bg_color.read(),
					);
					tx.send(MsgToGtk::Update).unwrap();
				}
			}
			STANDBY_SLEEP
		});
	});

	application.run(&[]);
}
