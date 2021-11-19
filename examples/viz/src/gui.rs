use crate::{drawer::DrawSettings, T};

use forceatlas2::*;
use gio::prelude::*;
use gtk::{
	prelude::*,
	traits::{EntryExt, SettingsExt},
};
use parking_lot::RwLock;
use rand::Rng;
use static_rc::StaticRc;
use std::{rc::Rc, sync::Arc, thread, time::Duration};

const STANDBY_SLEEP: Duration = Duration::from_millis(50);
const DRAW_SLEEP: Duration = Duration::from_millis(30);

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
	draw_settings: Arc<RwLock<DrawSettings>>,
	zoom: Arc<RwLock<T>>,
	d3: Arc<RwLock<bool>>,
	nb_iters: Arc<RwLock<usize>>,
) {
	let builder = gtk::Builder::new();
	builder.add_from_string(include_str!("gui.glade")).unwrap();

	let window: gtk::ApplicationWindow = builder.object("window").unwrap();
	window.set_application(Some(app));

	let graph_area: gtk::Image = builder.object("graph_area").unwrap();
	let graph_box: gtk::ScrolledWindow = builder.object("graph_box").unwrap();
	let graph_viewport: gtk::Viewport = builder.object("graph_viewport").unwrap();
	let graph_hadj: gtk::Adjustment = graph_box.hadjustment();
	let graph_vadj: gtk::Adjustment = graph_box.vadjustment();
	let compute_button: gtk::ToggleButton = builder.object("bt_compute").unwrap();
	let reset_button: gtk::Button = builder.object("bt_reset").unwrap();
	let save_img_button: gtk::Button = builder.object("bt_save_img").unwrap();
	let copy_img_button: gtk::Button = builder.object("bt_copy_img").unwrap();
	let chunk_size_input: gtk::Entry = builder.object("chunk_size").unwrap();
	let ka_input: gtk::Entry = builder.object("ka").unwrap();
	let kg_input: gtk::Entry = builder.object("kg").unwrap();
	let kr_input: gtk::Entry = builder.object("kr").unwrap();
	let speed_input: gtk::Entry = builder.object("speed").unwrap();
	let draw_edges_input: gtk::CheckButton = builder.object("draw_edges").unwrap();
	let edge_color_input: gtk::ColorButton = builder.object("edge_color").unwrap();
	let draw_nodes_input: gtk::CheckButton = builder.object("draw_nodes").unwrap();
	let node_color_input: gtk::ColorButton = builder.object("node_color").unwrap();
	let node_radius_input: gtk::Entry = builder.object("node_radius").unwrap();
	let bg_color_input: gtk::ColorButton = builder.object("bg_color").unwrap();
	let zoom_input: gtk::Entry = builder.object("zoom").unwrap();
	let d3_input: gtk::CheckButton = builder.object("3d").unwrap();
	let nb_iters_disp: gtk::Label = builder.object("nb_iters").unwrap();

	let save_img_window: gtk::FileChooserDialog = builder.object("save_img_window").unwrap();
	let siw_cancel_button: gtk::Button = builder.object("siw_bt_cancel").unwrap();
	let siw_save_button: gtk::Button = builder.object("siw_bt_save").unwrap();
	let siw_filename: gtk::Entry = builder.object("siw_filename").unwrap();
	let siw_filetype: gtk::ComboBox = builder.object("siw_filetype").unwrap();

	{
		let mut draw_settings = draw_settings.write();

		if window.settings().map_or(false, |s| {
			s.gtk_theme_name()
				.map_or(false, |s| s.as_str().ends_with("-dark"))
		}) {
			draw_settings.edge_color = (255, 255, 255, 20);
			draw_settings.node_color = (255, 127, 0);
			draw_settings.bg_color = (0, 0, 0);
		}

		{
			let settings = settings.read();
			chunk_size_input.set_text(&settings.chunk_size.unwrap_or(0).to_string());
			ka_input.set_text(&settings.ka.to_string());
			kg_input.set_text(&settings.kg.to_string());
			kr_input.set_text(&settings.kr.to_string());
			speed_input.set_text(&settings.speed.to_string());
		}
		draw_edges_input.set_active(draw_settings.draw_edges);
		edge_color_input.set_rgba({
			&gdk::RGBA {
				red: draw_settings.edge_color.0 as f64 / 255.,
				green: draw_settings.edge_color.1 as f64 / 255.,
				blue: draw_settings.edge_color.2 as f64 / 255.,
				alpha: draw_settings.edge_color.3 as f64 / 255.,
			}
		});
		draw_nodes_input.set_active(draw_settings.draw_nodes);
		node_color_input.set_rgba({
			&gdk::RGBA {
				red: draw_settings.node_color.0 as f64 / 255.,
				green: draw_settings.node_color.1 as f64 / 255.,
				blue: draw_settings.node_color.2 as f64 / 255.,
				alpha: 1.,
			}
		});
		node_radius_input.set_text(&draw_settings.node_radius.to_string());
		bg_color_input.set_rgba({
			&gdk::RGBA {
				red: draw_settings.bg_color.0 as f64 / 255.,
				green: draw_settings.bg_color.1 as f64 / 255.,
				blue: draw_settings.bg_color.2 as f64 / 255.,
				alpha: 1.,
			}
		});
	}
	zoom_input.set_text(&zoom.read().to_string());

	{
		let layout = layout.read();
		let nb_nodes_disp: gtk::Label = builder.object("nb_nodes").unwrap();
		let nb_edges_disp: gtk::Label = builder.object("nb_edges").unwrap();
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
			*graph_drag.write() = Some((graph_adj.0.value(), graph_adj.1.value()));
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
		let graph_adj = graph_adj.clone();
		move |_, x, y| {
			if let Some((cx, cy)) = graph_drag.write().take() {
				graph_adj.0.set_value(cx - x);
				graph_adj.1.set_value(cy - y);
			}
		}
	});

	graph_area.connect_key_press_event({
		let tx = tx.clone();
		let draw_settings = draw_settings.clone();
		let zoom_input = zoom_input.clone();
		let zoom = zoom.clone();
		move |_, event| {
			#[allow(clippy::no_effect, unused_must_use)]
			{
				&graph_gesture_drag; // avoid GC
			}

			if let Some(key) = event.keyval().name() {
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
					"KP_2" | "2" => {
						draw_settings.write().camera_angle.0 -= 0.1;
						tx.write().redraw = true;
					}
					"KP_4" | "4" => {
						draw_settings.write().camera_angle.1 -= 0.1;
						tx.write().redraw = true;
					}
					"KP_5" | "5" => {
						draw_settings.write().camera_angle = (0.0, 0.0);
						tx.write().redraw = true;
					}
					"KP_6" | "6" => {
						draw_settings.write().camera_angle.1 += 0.1;
						tx.write().redraw = true;
					}
					"KP_8" | "8" => {
						draw_settings.write().camera_angle.0 += 0.1;
						tx.write().redraw = true;
					}
					"Right" => {
						graph_adj.0.set_value(graph_adj.0.value() + 16.0);
						return Inhibit(true);
					}
					"Left" => {
						graph_adj.0.set_value(graph_adj.0.value() - 16.0);
						return Inhibit(true);
					}
					"Down" => {
						graph_adj.1.set_value(graph_adj.1.value() + 16.0);
						return Inhibit(true);
					}
					"Up" => {
						graph_adj.1.set_value(graph_adj.1.value() - 16.0);
						return Inhibit(true);
					}
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
			*compute = bt.is_active();
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
			let filename = siw_filename.text();
			let filename = filename.as_str();
			let filetype = siw_filetype.active_id();
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
				if let Some(current_folder) = save_img_window.current_folder() {
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
			if let Ok(chunk_size) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut settings = settings.write();
				settings.chunk_size = if chunk_size == 0 {
					None
				} else {
					Some(chunk_size)
				};
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	ka_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(ka) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut settings = settings.write();
				settings.ka = ka;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	kg_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(kg) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut settings = settings.write();
				settings.kg = kg;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	kr_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(kr) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut settings = settings.write();
				settings.kr = kr;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	speed_input.connect_changed({
		let layout = layout.clone();
		let settings = settings.clone();
		move |entry| {
			if let Ok(speed) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut settings = settings.write();
				settings.speed = speed;
				let mut layout = layout.write();
				layout.set_settings(settings.clone());
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	draw_edges_input.connect_toggled({
		let tx = tx.clone();
		let draw_settings = draw_settings.clone();
		move |draw_edges_input| {
			draw_settings.write().draw_edges = draw_edges_input.is_active();
			tx.write().redraw = true;
		}
	});

	edge_color_input.connect_color_set({
		let tx = tx.clone();
		let draw_settings = draw_settings.clone();
		move |edge_color_input| {
			let c = edge_color_input.rgba();
			draw_settings.write().edge_color = (
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
		let draw_settings = draw_settings.clone();
		move |draw_nodes_input| {
			draw_settings.write().draw_nodes = draw_nodes_input.is_active();
			tx.write().redraw = true;
		}
	});

	node_color_input.connect_color_set({
		let tx = tx.clone();
		let draw_settings = draw_settings.clone();
		move |node_color_input| {
			let c = node_color_input.rgba();
			draw_settings.write().node_color = (
				(c.red * 255.) as u8,
				(c.green * 255.) as u8,
				(c.blue * 255.) as u8,
			);
			tx.write().redraw = true;
		}
	});

	node_radius_input.connect_changed({
		let tx = tx.clone();
		let draw_settings = draw_settings.clone();
		move |entry| {
			if let Ok(v) = entry.text().parse() {
				entry.set_secondary_icon_name(None);
				let mut draw_settings = draw_settings.write();
				if draw_settings.node_radius != v {
					tx.write().redraw = true;
				}
				draw_settings.node_radius = v;
			} else {
				entry.set_secondary_icon_name(Some("emblem-unreadable"));
			}
		}
	});

	bg_color_input.connect_color_set({
		let tx = tx.clone();
		move |bg_color_input| {
			let c = bg_color_input.rgba();
			draw_settings.write().bg_color = (
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
			if let Ok(val) = entry.text().parse() {
				if val > 0.0 {
					entry.set_secondary_icon_name(None);
					let mut zoom = zoom.write();
					let mut pixbuf = pixbuf.write();
					*zoom = val;
					*pixbuf = gdk_pixbuf::Pixbuf::new(
						gdk_pixbuf::Colorspace::Rgb,
						false,
						8,
						(graph_viewport.allocated_width() as T * *zoom) as i32,
						(graph_viewport.allocated_height() as T * *zoom) as i32,
					)
					.map(Pixbuf);
					tx.write().redraw = true;
					return;
				}
			}
			entry.set_secondary_icon_name(Some("emblem-unreadable"));
		}
	});

	d3_input.connect_toggled({
		let tx = tx.clone();
		let nb_iters = nb_iters.clone();
		move |d3_input| {
			let mut d3 = d3.write();
			let mut layout = layout.write();
			let mut settings = settings.write();
			*d3 = d3_input.is_active();
			settings.dimensions = if *d3 { 3 } else { 2 };
			*layout = Layout::from_graph(
				layout.edges.clone(),
				Nodes::Degree(layout.masses.len()),
				layout.weights.clone(),
				settings.clone(),
			);
			*nb_iters.write() = 0;
			tx.write().redraw = true;
			drop(layout);
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
				(graph_viewport.allocated_width() as T * *zoom) as i32,
				(graph_viewport.allocated_height() as T * *zoom) as i32,
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
	);

	let (tx, rx) = glib::MainContext::sync_channel(glib::PRIORITY_DEFAULT, 4);
	let rx = Arc::new(RwLock::new(Some(rx)));
	let msg_from_gtk = Arc::new(RwLock::new(MsgFromGtk {
		redraw: true,
		resize: false,
	}));
	let pixbuf = Arc::new(RwLock::new(None));
	let draw_settings = Arc::new(RwLock::new(DrawSettings {
		draw_edges: true,
		edge_color: (0, 0, 0, 20),
		draw_nodes: true,
		node_color: (255, 0, 0),
		node_radius: 2,
		bg_color: (255, 255, 255),
		camera_angle: (0.0, 0.0),
	}));
	let zoom = Arc::new(RwLock::new(1.0));
	let d3 = Arc::new(RwLock::new(false));

	application.connect_activate({
		let compute = compute.clone();
		let layout = layout.clone();
		let pixbuf = pixbuf.clone();
		let draw_settings = draw_settings.clone();
		let msg_from_gtk = msg_from_gtk.clone();
		let d3 = d3.clone();
		move |app| {
			build_ui(
				app,
				rx.clone(),
				msg_from_gtk.clone(),
				compute.clone(),
				layout.clone(),
				settings.clone(),
				pixbuf.clone(),
				draw_settings.clone(),
				zoom.clone(),
				d3.clone(),
				nb_iters.clone(),
			)
		}
	});

	thread::spawn(move || loop {
		thread::sleep(if *compute.read() {
			if let Some(pixbuf) = pixbuf.write().as_ref() {
				let layout = layout.read();
				if *d3.read() {
					crate::drawer::draw_graph_3d(
						layout,
						(pixbuf.0.width(), pixbuf.0.height()),
						unsafe { pixbuf.0.pixels() },
						pixbuf.0.rowstride(),
						draw_settings.read().clone(),
					);
				} else {
					crate::drawer::draw_graph(
						layout,
						(pixbuf.0.width(), pixbuf.0.height()),
						unsafe { pixbuf.0.pixels() },
						pixbuf.0.rowstride(),
						draw_settings.read().clone(),
					);
				}
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
					if *d3.read() {
						crate::drawer::draw_graph_3d(
							layout,
							(pixbuf.0.width(), pixbuf.0.height()),
							unsafe { pixbuf.0.pixels() },
							pixbuf.0.rowstride(),
							draw_settings.read().clone(),
						);
					} else {
						crate::drawer::draw_graph(
							layout,
							(pixbuf.0.width(), pixbuf.0.height()),
							unsafe { pixbuf.0.pixels() },
							pixbuf.0.rowstride(),
							draw_settings.read().clone(),
						);
					}
					tx.send(MsgToGtk::Update).unwrap();
				}
			}
			STANDBY_SLEEP
		});
	});

	application.run_with_args::<&str>(&[]);
}
