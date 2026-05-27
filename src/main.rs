use relm4::prelude::*;
use relm4::gtk;
use gtk::prelude::*;

use dexgluco::core::{get_sensors, connect, monitor, Sensor, Connection, GlucoseReading};
use dexgluco::io::Task;
use dexgluco::io::qr::ScanDataMatrix;
use dexgluco::io::storage::LoadSensors;
use dexgluco::io::ble::MonitorSensor;

// ============================== Model ==============================

#[derive(Default)]
struct AppModel {
    log: String,
    serial: String,
    pin: String,
    address: String,
    qr_path: String,
    db_path: String,
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
    GetSensors,
    Connect,
    Monitor,
    MonitorReal,
    ScanQr,
    SerialChanged(String),
    PinChanged(String),
    AddressChanged(String),
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
        serial_entry.set_text("DXCM123456");

        let pin_entry = gtk::Entry::new();
        pin_entry.set_hexpand(true);
        pin_entry.set_text("123456");

        let address_entry = gtk::Entry::new();
        address_entry.set_hexpand(true);
        address_entry.set_text("00:11:22:33:44:55");

        let qr_entry = gtk::Entry::new();
        qr_entry.set_hexpand(true);
        qr_entry.set_text("tests/data/sensor-qr.jpg");

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

        let get_btn = gtk::Button::with_label("Get Sensors");
        let connect_btn = gtk::Button::with_label("Connect");
        let monitor_btn = gtk::Button::with_label("Monitor");
        let monitor_real_btn = gtk::Button::with_label("Monitor Real");
        let scan_btn = gtk::Button::with_label("Scan QR");
        let clear_btn = gtk::Button::with_label("Clear Log");

        let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        btn_box.set_homogeneous(true);
        btn_box.append(&get_btn);
        btn_box.append(&connect_btn);
        btn_box.append(&monitor_btn);
        btn_box.append(&monitor_real_btn);
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

        get_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::GetSensors)
        });
        connect_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::Connect)
        });
        monitor_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::Monitor)
        });
        monitor_real_btn.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::MonitorReal)
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

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AppMsg, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Log(s) => self.logln(s),
            AppMsg::ClearLog => self.log.clear(),

            AppMsg::SerialChanged(s) => self.serial = s,
            AppMsg::PinChanged(s) => self.pin = s,
            AppMsg::AddressChanged(s) => self.address = s,
            AppMsg::QrPathChanged(s) => self.qr_path = s,
            AppMsg::DbPathChanged(s) => self.db_path = s,

            AppMsg::GetSensors => {
                self.logln("▶ Get Sensors");
                let serial = self.serial.clone();
                let pin = self.pin.clone();
                let address = self.address.clone();
                let db_path = self.db_path.clone();
                let input = sender.input_sender().clone();

                tokio::spawn(async move {
                    input.send(AppMsg::Log("  Trying storage...".into())).ok();
                    let result = get_sensors(
                        {
                            let p = db_path.clone();
                            move || LoadSensors::new(p.clone()).run()
                        },
                        {
                            let input = input.clone();
                            let serial = serial.clone();
                            let pin = pin.clone();
                            let address = address.clone();
                            move || {
                                input.send(AppMsg::Log("  Storage empty — creating mock sensor".into())).ok();
                                Task::from_value(Sensor {
                                    serial: serial.clone(),
                                    pin: pin.clone(),
                                    address: address.clone(),
                                    shared_key: None,
                                })
                            }
                        },
                    ).run().await;

                    match result {
                        Ok(sensors) => {
                            input.send(AppMsg::Log(format!("  Got {} sensor(s)", sensors.len()))).ok();
                            for s in &sensors {
                                input.send(AppMsg::Log(
                                    format!("    {} @ {}", s.serial, s.address)
                                )).ok();
                            }
                            input.send(AppMsg::Log("✓ Get Sensors OK".into())).ok();
                        }
                        Err(e) => {
                            input.send(AppMsg::Log(format!("✗ Get Sensors error: {}", e))).ok();
                        }
                    }
                });
            }

            AppMsg::Connect => {
                self.logln("▶ Connect");
                let serial = self.serial.clone();
                let pin = self.pin.clone();
                let address = self.address.clone();
                let input = sender.input_sender().clone();

                tokio::spawn(async move {
                    let sensor = Sensor { serial, pin, address, shared_key: None };
                    input.send(AppMsg::Log(format!("  Connecting to {}...", sensor.serial))).ok();

                    let result = connect(
                        move |s| {
                            Task::from_value(Connection {
                                sensor: s,
                                stream: vec![],
                            })
                        },
                        vec![sensor],
                    ).run().await;

                    match result {
                        Ok(conns) => {
                            input.send(AppMsg::Log(format!("  Got {} connection(s)", conns.len()))).ok();
                            for c in &conns {
                                input.send(AppMsg::Log(
                                    format!("    {} — {} cached readings", c.sensor.serial, c.stream.len())
                                )).ok();
                            }
                            input.send(AppMsg::Log("✓ Connect OK".into())).ok();
                        }
                        Err(e) => {
                            input.send(AppMsg::Log(format!("✗ Connect error: {}", e))).ok();
                        }
                    }
                });
            }

            AppMsg::Monitor => {
                self.logln("▶ Monitor (mock via core::monitor)");

                let sensor = Sensor {
                    serial: self.serial.clone(),
                    pin: self.pin.clone(),
                    address: self.address.clone(),
                    shared_key: None,
                };
                let input = sender.input_sender().clone();
                let mock_readings: Vec<GlucoseReading> = vec![
                    GlucoseReading { value: 142.0, timestamp: 1700000000, trend: 3 },
                    GlucoseReading { value: 138.0, timestamp: 1700000005, trend: 2 },
                    GlucoseReading { value: 145.0, timestamp: 1700000010, trend: 4 },
                    GlucoseReading { value: 151.0, timestamp: 1700000015, trend: 5 },
                    GlucoseReading { value: 147.0, timestamp: 1700000020, trend: 2 },
                ];

                tokio::spawn(async move {
                    let input_sensor = input.clone();
                    let readings = mock_readings.clone();
                    let run_sensor = move |_: Sensor| {
                        let input = input_sensor.clone();
                        let readings = readings.clone();
                        Task::new(async move {
                            for r in &readings {
                                input.send(AppMsg::Log(
                                    format!("  Glucose: {} mg/dL  trend: {}  ts: {}",
                                        r.value, r.trend, r.timestamp)
                                )).ok();
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                            }
                            input.send(AppMsg::Log("✓ Mock sensor done".into())).ok();
                            loop {
                                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                            }
                        })
                    };

                    match monitor(vec![sensor], run_sensor).await {
                        Ok(()) => input.send(AppMsg::Log("✓ Monitor completed".into())).ok(),
                        Err(e) => input.send(AppMsg::Log(format!("✗ Monitor error: {}", e))).ok(),
                    }
                });
            }

            AppMsg::MonitorReal => {
                self.logln("▶ Monitor Real (BLE)");

                let sensor = Sensor {
                    serial: self.serial.clone(),
                    pin: self.pin.clone(),
                    address: self.address.clone(),
                    shared_key: None,
                };
                let input = sender.input_sender().clone();

                tokio::spawn(async move {
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

                    match MonitorSensor::new(sensor).run(on_reading).await {
                        Ok(()) => input.send(AppMsg::Log("✓ Real monitor ended".into())).ok(),
                        Err(e) => input.send(AppMsg::Log(format!("✗ Real monitor error: {}", e))).ok(),
                    }
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
    }
}

// ============================== Main ==============================

fn main() {
    let app = RelmApp::new("com.dexgluco.testui");
    app.run::<AppModel>(());
}
