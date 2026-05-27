use relm4::prelude::*;
use relm4::gtk;
use gtk::prelude::*;

use dexgluco::core::{Sensor, GlucoseReading};
use dexgluco::io::qr::ScanDataMatrix;
use dexgluco::io::storage::{LoadSensors, SaveSensor};
use dexgluco::io::ble::{MonitorSensor, ScanForSensor};

// ============================== Model ==============================

struct AppModel {
    log: String,
    serial: String,
    pin: String,
    address: String,
    shared_key: Option<[u8; 16]>,
    qr_path: String,
    db_path: String,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            log: String::new(),
            serial: String::new(),
            pin: String::new(),
            address: String::new(),
            shared_key: None,
            qr_path: "/home/lukas/dev/dexgluco/tests/data/sensor-qr.jpg".into(),
            db_path: "sensors.db".into(),
        }
    }
}

impl AppModel {
    fn logln(&mut self, s: impl AsRef<str>) {
        self.log.push_str(s.as_ref());
        self.log.push('\n');
    }
}

// ============================== Messages ==============================

#[derive(Debug)]
enum AppMsg {
    Log(String),
    ClearLog,
    Monitor,
    ScanQr,
    SerialChanged(String),
    PinChanged(String),
    AddressChanged(String),
    SharedKeyChanged(Option<[u8; 16]>),
    QrPathChanged(String),
    DbPathChanged(String),
}

// ============================== Widgets ==============================

#[derive(Debug)]
struct AppWidgets {
    main_window: gtk::Window,
    log_view: gtk::TextView,
    serial_entry: gtk::Entry,
    pin_entry: gtk::Entry,
    address_entry: gtk::Entry,
    qr_entry: gtk::Entry,
    status_label: gtk::Label,
}

// ============================== Component ==============================

impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Root = gtk::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::new()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let log_view = gtk::TextView::new();
        log_view.set_editable(false);
        log_view.set_monospace(true);
        log_view.set_cursor_visible(false);
        log_view.set_wrap_mode(gtk::WrapMode::WordChar);

        let serial_entry = gtk::Entry::new();
        serial_entry.set_hexpand(true);

        let pin_entry = gtk::Entry::new();
        pin_entry.set_hexpand(true);

        let address_entry = gtk::Entry::new();
        address_entry.set_hexpand(true);

        let qr_entry = gtk::Entry::new();
        qr_entry.set_hexpand(true);
        qr_entry.set_text("/home/lukas/dev/dexgluco/tests/data/sensor-qr.jpg");

        let status_label = gtk::Label::new(Some("Ready"));
        status_label.set_xalign(0.0);
        status_label.set_selectable(true);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.set_min_content_height(250);
        scrolled.set_child(Some(&log_view));

        let row0 = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row0.set_margin_top(4);
        row0.set_margin_bottom(4);
        row0.append(&gtk::Label::new(Some("Serial:")));
        row0.append(&serial_entry);
        row0.append(&gtk::Label::new(Some("PIN:")));
        row0.append(&pin_entry);

        let row1 = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row1.append(&gtk::Label::new(Some("Address:")));
        row1.append(&address_entry);

        let row2 = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row2.append(&gtk::Label::new(Some("QR Path:")));
        row2.append(&qr_entry);

        let monitor_btn = gtk::Button::with_label("Monitor");
        let scan_btn = gtk::Button::with_label("Scan QR");
        let clear_btn = gtk::Button::with_label("Clear Log");

        let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        btn_box.set_homogeneous(true);
        btn_box.append(&monitor_btn);
        btn_box.append(&scan_btn);
        btn_box.append(&clear_btn);

        let sep = gtk::Separator::new(gtk::Orientation::Horizontal);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 6);
        vbox.set_margin_all(8);
        vbox.append(&scrolled);
        vbox.append(&sep);
        vbox.append(&row0);
        vbox.append(&row1);
        vbox.append(&row2);
        vbox.append(&btn_box);
        vbox.append(&status_label);

        root.set_title(Some("Dexgluco — Test UI"));
        root.set_default_size(750, 550);
        root.set_child(Some(&vbox));

        serial_entry.connect_changed({
            let sender = sender.clone();
            move |e| sender.input(AppMsg::SerialChanged(e.text().to_string()))
        });
        pin_entry.connect_changed({
            let sender = sender.clone();
            move |e| sender.input(AppMsg::PinChanged(e.text().to_string()))
        });
        address_entry.connect_changed({
            let sender = sender.clone();
            move |e| sender.input(AppMsg::AddressChanged(e.text().to_string()))
        });
        qr_entry.connect_changed({
            let sender = sender.clone();
            move |e| sender.input(AppMsg::QrPathChanged(e.text().to_string()))
        });

        monitor_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::Monitor)
        });
        scan_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ScanQr)
        });
        clear_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ClearLog)
        });

        let widgets = AppWidgets {
            main_window: root,
            log_view,
            serial_entry,
            pin_entry,
            address_entry,
            qr_entry,
            status_label,
        };

        let model = AppModel::default();

        let input = sender.input_sender().clone();
        let db_path = model.db_path.clone();
        tokio::spawn(async move {
            match LoadSensors::new(db_path).run().await {
                Ok(sensors) => {
                    if let Some(s) = sensors.into_iter().next() {
                        input.send(AppMsg::SerialChanged(s.serial)).ok();
                        input.send(AppMsg::PinChanged(s.pin)).ok();
                        input.send(AppMsg::AddressChanged(s.address)).ok();
                        input.send(AppMsg::SharedKeyChanged(s.shared_key)).ok();
                    }
                }
                Err(_) => {}
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Log(s) => self.logln(s),
            AppMsg::ClearLog => self.log.clear(),

            AppMsg::SerialChanged(s) => self.serial = s,
            AppMsg::PinChanged(s) => self.pin = s,
            AppMsg::AddressChanged(s) => self.address = s,
            AppMsg::SharedKeyChanged(k) => {
                self.shared_key = k;
                if k.is_some() {
                    self.logln("  Loaded shared key from DB (bonded)");
                }
            }
            AppMsg::QrPathChanged(s) => self.qr_path = s,
            AppMsg::DbPathChanged(s) => self.db_path = s,

            AppMsg::Monitor => {
                self.logln("▶ Monitor (BLE)");

                let serial = self.serial.clone();
                let pin = self.pin.clone();
                let mut address = self.address.clone();
                let shared_key = self.shared_key;
                let db_path = self.db_path.clone();
                let input = sender.input_sender().clone();

                tokio::spawn(async move {
                    let device = if address.is_empty() {
                        input.send(AppMsg::Log("  Scanning BLE for sensor...".into())).ok();
                        match ScanForSensor(serial.clone()).run().await {
                            Ok((dev, addr)) => {
                                input.send(AppMsg::AddressChanged(addr.clone())).ok();
                                address = addr.clone();

                                let s = Sensor {
                                    serial: serial.clone(),
                                    pin: pin.clone(),
                                    address: addr,
                                    shared_key: None,
                                };
                                SaveSensor::new(db_path.clone(), s).run().await.ok();
                                input.send(AppMsg::Log("  Saved sensor to DB".into())).ok();
                                Some(dev)
                            }
                            Err(e) => {
                                input.send(AppMsg::Log(format!("✗ BLE scan error: {}", e))).ok();
                                return;
                            }
                        }
                    } else {
                        None
                    };

                    let sensor = Sensor { serial: serial.clone(), pin: pin.clone(), address: address.clone(), shared_key };
                    input.send(AppMsg::Log(format!(
                        "  Connecting to {} @ {} ...", sensor.serial, sensor.address
                    ))).ok();

                    let input_cb = input.clone();
                    let on_reading = move |r: GlucoseReading| {
                        input_cb.send(AppMsg::Log(format!(
                            "  EGV: {} mg/dL  trend: {}  ts: {}",
                            r.value, r.trend, r.timestamp
                        ))).ok();
                    };

                    let input_cb2 = input.clone();
                    let db_path2 = db_path.clone();
                    let serial2 = serial.clone();
                    let pin2 = pin.clone();
                    let address2 = address.clone();
                    let on_auth = move |shared_key: [u8; 16]| {
                        let sensor = Sensor {
                            serial: serial2.clone(),
                            pin: pin2.clone(),
                            address: address2.clone(),
                            shared_key: Some(shared_key),
                        };
                        input_cb2.send(AppMsg::Log("  Authenticated — saving shared key".into())).ok();
                        let db = db_path2.clone();
                        tokio::spawn(async move {
                            SaveSensor::new(db, sensor).run().await.ok();
                        });
                    };

                    let mut monitor = MonitorSensor::new(sensor);
                    monitor.device = device;
                    match monitor.run(on_reading, on_auth).await {
                        Ok(()) => input.send(AppMsg::Log("✓ Real monitor ended".into())).ok(),
                        Err(e) => input.send(AppMsg::Log(format!("✗ Real monitor error: {}", e))).ok(),
                    };
                });
            }

            AppMsg::ScanQr => {
                self.logln("▶ Scan QR");
                let path = self.qr_path.clone();
                let input = sender.input_sender().clone();

                tokio::spawn(async move {
                    input.send(AppMsg::Log(format!("  Decoding: {}", path))).ok();

                    match ScanDataMatrix(path).run().await {
                        Ok((serial, pin)) => {
                            input.send(AppMsg::SerialChanged(serial.clone())).ok();
                            input.send(AppMsg::PinChanged(pin.clone())).ok();
                            input.send(AppMsg::Log(
                                format!("✓ QR decoded — serial: {}, PIN: {}", serial, pin)
                            )).ok();
                        }
                        Err(e) => {
                            input.send(AppMsg::Log(format!("✗ QR error: {}", e))).ok();
                        }
                    }
                });
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let buf = widgets.log_view.buffer();
        buf.set_text(&self.log);
        let mut end = buf.end_iter();
        widgets.log_view.scroll_to_iter(&mut end, 0.0, true, 0.0, 0.0);
        let status = self.log.lines().last().unwrap_or("Ready");
        widgets.status_label.set_text(status);

        if widgets.serial_entry.text().as_str() != self.serial {
            widgets.serial_entry.set_text(&self.serial);
        }
        if widgets.pin_entry.text().as_str() != self.pin {
            widgets.pin_entry.set_text(&self.pin);
        }
        if widgets.address_entry.text().as_str() != self.address {
            widgets.address_entry.set_text(&self.address);
        }
        if widgets.qr_entry.text().as_str() != self.qr_path {
            widgets.qr_entry.set_text(&self.qr_path);
        }
    }
}

// ============================== Main ==============================

fn main() {
    let app = RelmApp::new("com.dexgluco.testui");
    app.run::<AppModel>(());
}
