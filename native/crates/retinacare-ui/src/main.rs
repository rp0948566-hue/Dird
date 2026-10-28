#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};
use retinacare_app::{
    WorkerSupervisor,
    database::{DatabaseAction, DatabaseController, DatabaseSnapshot},
    resources::ResourcePaths,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    Patients,
    Screening,
    History,
    Results,
    Reports,
    Settings,
    Gpu,
}

struct RetinaCareUi {
    screen: Screen,
    worker: WorkerSupervisor,
    worker_status: Value,
    image_path: String,
    image_texture: Option<TextureHandle>,
    image_error: Option<String>,
    zoom: f32,
    pan: Vec2,
    image_load: Option<(String, Receiver<Result<image::RgbaImage, String>>)>,
    loaded_image_path: String,
    last_analysis: Value,
    database: DatabaseController,
    records: DatabaseSnapshot,
    database_error: Option<String>,
    password: Zeroizing<String>,
    patient_code: String,
    patient_name: String,
    patient_search: String,
    editing_patient: Option<i64>,
    selected_patient: Option<i64>,
    selected_examination: Option<i64>,
    exam_operator: String,
    exam_notes: String,
    eye: String,
    notice: Option<String>,
    smoke: Option<(PathBuf, Instant, u32)>,
    screenshot: Option<(PathBuf, u32, bool)>,
}

impl RetinaCareUi {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = Color32::from_rgb(246, 247, 249);
        visuals.window_fill = Color32::from_rgb(250, 250, 248);
        visuals.selection.bg_fill = Color32::from_rgb(41, 112, 185);
        context.egui_ctx.set_visuals(visuals);
        Self {
            screen: Screen::Dashboard,
            worker: WorkerSupervisor::start_default(),
            worker_status: serde_json::json!({"status":"starting"}),
            image_path: String::new(),
            image_texture: None,
            image_error: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            image_load: None,
            loaded_image_path: String::new(),
            last_analysis: Value::Null,
            database: DatabaseController::default(),
            records: DatabaseSnapshot::default(),
            database_error: None,
            password: Zeroizing::new(String::new()),
            patient_code: String::new(),
            patient_name: String::new(),
            patient_search: String::new(),
            editing_patient: None,
            selected_patient: None,
            selected_examination: None,
            exam_operator: String::new(),
            exam_notes: String::new(),
            eye: "left".into(),
            notice: None,
            smoke: std::env::args_os()
                .skip_while(|arg| arg != "--gui-smoke-test")
                .nth(1)
                .map(|path| (PathBuf::from(path), Instant::now(), 0)),
            screenshot: std::env::args_os()
                .skip_while(|arg| arg != "--screenshot")
                .nth(1)
                .map(|path| (PathBuf::from(path), 0, false)),
        }
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(Color32::from_rgb(31, 38, 46))
            .inner_margin(egui::Margin::same(18))
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(Color32::from_rgb(229, 234, 240));
                ui.heading("RetinaCare");
                ui.label("AI Manthan PS14");
                ui.add_space(16.0);
                for (label, screen) in [
                    ("Dashboard", Screen::Dashboard),
                    ("Patients", Screen::Patients),
                    ("New screening", Screen::Screening),
                    ("Examination history", Screen::History),
                    ("AI results", Screen::Results),
                    ("Medical reports", Screen::Reports),
                    ("Settings", Screen::Settings),
                    ("GPU and models", Screen::Gpu),
                ] {
                    if ui
                        .add_sized(
                            [190.0, 34.0],
                            egui::Button::selectable(self.screen == screen, label),
                        )
                        .clicked()
                    {
                        self.screen = screen;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.small("Clinician review required");
                    ui.small("Offline • encrypted • local");
                });
            });
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Screening overview");
        ui.label("Offline retinal-screening workspace and encrypted clinical records.");
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            status_card(
                ui,
                "Worker",
                self.worker_status["status"].as_str().unwrap_or("starting"),
            );
            status_card(ui, "GPU", device_name(&self.worker_status));
            status_card(
                ui,
                "Models",
                self.worker_status["data"]["model_state"]
                    .as_str()
                    .unwrap_or("checking"),
            );
        });
        ui.add_space(18.0);
        ui.columns(2, |columns| {
            columns[0].group(|ui| {
                ui.strong("Clinical workspace");
                ui.label(format!("Encrypted database: {}", if self.records.unlocked { "unlocked" } else { "locked" }));
                ui.label(format!("Patients: {}", self.records.patients.len()));
                ui.label(format!("Saved examinations: {}", self.records.examinations.len()));
                ui.add_space(8.0);
                ui.strong("Recent examinations");
                if self.records.examinations.is_empty() {
                    ui.label("No saved examinations yet.");
                }
                for exam in self.records.examinations.iter().take(4) {
                    ui.label(format!("{} • {} eye • {}", exam.examined_at, if exam.eye.is_empty() { "unknown" } else { &exam.eye }, exam.operator));
                }
            });
            columns[1].group(|ui| {
                ui.strong("Quick actions");
                if ui.button("Open patient registry").clicked() { self.screen = Screen::Patients; }
                if ui.add_enabled(self.records.unlocked, egui::Button::new("Start new screening")).clicked() { self.screen = Screen::Screening; }
                if ui.button("Refresh worker health").clicked() { self.worker.refresh(); }
                ui.add_space(8.0);
                ui.strong("Clinical boundary");
                ui.label("AI findings remain unavailable until compatible, validated model weights and an inference adapter are installed.");
            });
        });
    }

    fn database_action(&mut self, action: DatabaseAction) {
        self.database_error = self.database.send(action).err();
    }

    fn patient_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Patient registry");
        ui.label(
            "Encrypted local records. Use synthetic data until clinical validation is complete.",
        );
        ui.add_space(10.0);
        if !self.records.unlocked {
            ui.label("Enter a password to open your encrypted database. First use creates a new database.");
            ui.add(
                egui::TextEdit::singleline(&mut *self.password)
                    .password(true)
                    .hint_text("Database password"),
            );
            if ui.button("Open / create encrypted database").clicked() {
                let password = std::mem::replace(&mut self.password, Zeroizing::new(String::new()));
                self.database_action(DatabaseAction::Open(password));
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(format!("{} patients", self.records.patients.len()));
                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.patient_search)
                        .hint_text("Search code or name"),
                );
            });
            if ui.small_button("Lock encrypted database").clicked() {
                self.records = DatabaseSnapshot::default();
                self.patient_name.clear();
                self.patient_code.clear();
                self.exam_notes.clear();
                self.editing_patient = None;
                self.database_action(DatabaseAction::Lock);
            }
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.strong(if self.editing_patient.is_some() {
                    "Edit patient"
                } else {
                    "Add patient"
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.patient_code)
                            .hint_text("Patient code"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.patient_name)
                            .hint_text("Patient name"),
                    );
                    let label = if self.editing_patient.is_some() {
                        "Save changes"
                    } else {
                        "Add patient"
                    };
                    if ui.button(label).clicked() {
                        let action = match self.editing_patient {
                            Some(id) => DatabaseAction::UpdatePatient(
                                id,
                                self.patient_code.clone(),
                                self.patient_name.clone(),
                            ),
                            None => DatabaseAction::AddPatient(
                                self.patient_code.clone(),
                                self.patient_name.clone(),
                            ),
                        };
                        self.database_action(action);
                        self.patient_code.clear();
                        self.patient_name.clear();
                        self.editing_patient = None;
                    }
                    if self.editing_patient.is_some() && ui.button("Cancel").clicked() {
                        self.editing_patient = None;
                        self.patient_code.clear();
                        self.patient_name.clear();
                    }
                });
            });
            ui.add_space(8.0);
            let query = self.patient_search.trim().to_lowercase();
            for patient in self.records.patients.clone().into_iter().filter(|patient| {
                query.is_empty()
                    || patient.patient_code.to_lowercase().contains(&query)
                    || patient.display_name.to_lowercase().contains(&query)
            }) {
                ui.horizontal(|ui| {
                    ui.strong(&patient.patient_code);
                    ui.label(&patient.display_name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Open records").clicked() {
                            self.selected_patient = Some(patient.id);
                            self.screen = Screen::History;
                        }
                        if ui.button("Edit").clicked() {
                            self.editing_patient = Some(patient.id);
                            self.patient_code = patient.patient_code.clone();
                            self.patient_name = patient.display_name.clone();
                        }
                    });
                });
                ui.separator();
            }
        }
        if let Some(error) = &self.database_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    }

    fn history_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Examination history");
        if !self.records.unlocked {
            ui.label("Open encrypted storage from Patients to view history.");
            return;
        }
        egui::ComboBox::from_label("Patient")
            .selected_text(
                self.selected_patient
                    .map(|id| format!("Patient #{id}"))
                    .unwrap_or_else(|| "Select patient".into()),
            )
            .show_ui(ui, |ui| {
                for patient in &self.records.patients {
                    ui.selectable_value(
                        &mut self.selected_patient,
                        Some(patient.id),
                        &patient.patient_code,
                    );
                }
            });
        if let Some(id) = self.selected_patient {
            if ui.button("Create new screening").clicked() {
                self.screen = Screen::Screening;
            }
            for examination in self
                .records
                .examinations
                .clone()
                .into_iter()
                .filter(|exam| exam.patient_id == id)
            {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(&examination.examined_at);
                    ui.label(if examination.eye.is_empty() {
                        "Eye not recorded"
                    } else {
                        &examination.eye
                    });
                    if ui.button("Open report").clicked() {
                        self.selected_examination = examination.id;
                        self.screen = Screen::Reports;
                    }
                    if !examination.image_path.is_empty() && ui.button("Reopen image").clicked() {
                        self.image_path = examination.image_path.clone();
                        self.exam_operator = examination.operator.clone();
                        self.exam_notes = examination.notes.clone();
                        self.eye = if examination.eye.is_empty() {
                            "left".into()
                        } else {
                            examination.eye.clone()
                        };
                        self.screen = Screen::Screening;
                    }
                });
                ui.label(format!("Examiner: {}", examination.operator));
                ui.label(&examination.notes);
            }
        }
        if let Some(error) = &self.database_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
    }

    fn screening_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("New retinal screening");
        if !self.records.unlocked {
            ui.label("Open encrypted storage from Patients before creating a screening.");
            return;
        }
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Patient")
                .selected_text(
                    self.selected_patient
                        .and_then(|id| {
                            self.records
                                .patients
                                .iter()
                                .find(|patient| patient.id == id)
                        })
                        .map(|patient| {
                            format!("{} — {}", patient.patient_code, patient.display_name)
                        })
                        .unwrap_or_else(|| "Select patient".into()),
                )
                .show_ui(ui, |ui| {
                    for patient in &self.records.patients {
                        ui.selectable_value(
                            &mut self.selected_patient,
                            Some(patient.id),
                            format!("{} — {}", patient.patient_code, patient.display_name),
                        );
                    }
                });
            ui.label("Eye");
            ui.radio_value(&mut self.eye, "left".into(), "Left");
            ui.radio_value(&mut self.eye, "right".into(), "Right");
        });
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.exam_operator).hint_text("Examiner"));
            ui.add(egui::TextEdit::singleline(&mut self.exam_notes).hint_text("Clinical notes"));
        });
        ui.horizontal(|ui| {
            ui.label("Local fundus image path");
            ui.text_edit_singleline(&mut self.image_path);
            if ui.button("Browse…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Select retinal fundus image")
                    .add_filter(
                        "Retinal images",
                        &["png", "jpg", "jpeg", "tif", "tiff", "bmp"],
                    )
                    .pick_file()
            {
                self.image_path = path.to_string_lossy().into_owned();
                self.load_image(ui.ctx());
            }
            if ui
                .add_enabled(self.image_load.is_none(), egui::Button::new("Load image"))
                .clicked()
            {
                self.load_image(ui.ctx());
            }
        });
        if let Some(error) = &self.image_error {
            ui.colored_label(Color32::LIGHT_RED, error);
        }
        ui.horizontal(|ui| {
            ui.label("Zoom");
            ui.add(egui::Slider::new(&mut self.zoom, 0.2..=5.0));
            if ui.button("Reset view").clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }
            if ui
                .add_enabled(
                    self.worker_status["status"] == "ok" && self.image_texture.is_some(),
                    egui::Button::new("Analyze image"),
                )
                .clicked()
            {
                self.worker.analyze_image(self.loaded_image_path.clone());
                self.notice = Some("AI worker request started.".into());
            }
            let can_save = self.selected_patient.is_some() && self.image_texture.is_some();
            if ui
                .add_enabled(can_save, egui::Button::new("Save examination"))
                .clicked()
                && let Some(patient_id) = self.selected_patient
            {
                let analysis = if self.last_analysis.is_null() {
                    String::new()
                } else {
                    self.last_analysis.to_string()
                };
                self.database_action(DatabaseAction::AddExamination(
                    patient_id,
                    self.exam_operator.clone(),
                    self.exam_notes.clone(),
                    self.eye.clone(),
                    self.loaded_image_path.clone(),
                    analysis,
                ));
                self.notice = Some("Examination queued for encrypted save.".into());
            }
        });
        if let Some(notice) = &self.notice {
            ui.colored_label(Color32::from_rgb(28, 104, 159), notice);
        }
        let size = Vec2::new(
            ui.available_width().max(1.0),
            ui.available_height().max(1.0),
        );
        let (viewport, response) = ui.allocate_exact_size(size, Sense::drag());
        if response.dragged() {
            self.pan += ui.input(|input| input.pointer.delta());
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.zoom = (self.zoom * (scroll * 0.001).exp()).clamp(0.2, 5.0);
            }
        }
        ui.painter()
            .rect_filled(viewport, 4.0, Color32::from_rgb(18, 25, 32));
        if let Some(texture) = &self.image_texture {
            let image_size = texture.size_vec2() * self.zoom;
            let image_rect = Rect::from_center_size(viewport.center() + self.pan, image_size);
            ui.painter().with_clip_rect(viewport).image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                viewport.center(),
                egui::Align2::CENTER_CENTER,
                "Load a local fundus image",
                egui::FontId::proportional(18.0),
                Color32::LIGHT_GRAY,
            );
        }
        ui.label("Drag to pan • scroll to zoom • Missing model weights produce an explicit error, never a diagnosis.");
    }

    fn load_image(&mut self, _context: &egui::Context) {
        self.image_error = None;
        self.image_texture = None;
        self.loaded_image_path.clear();
        let path = self.image_path.clone();
        let worker_path = path.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = tx.send(decode_image(&worker_path));
        });
        self.image_load = Some((path, rx));
    }

    fn poll_image(&mut self, context: &egui::Context) {
        let update = self
            .image_load
            .as_ref()
            .and_then(|(_, rx)| match rx.try_recv() {
                Ok(update) => Some(update),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("Image loader exited".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(update) = update
            && let Some((path, _)) = self.image_load.take()
        {
            match update {
                Ok(rgba) => {
                    let dimensions = [rgba.width() as usize, rgba.height() as usize];
                    let color = egui::ColorImage::from_rgba_unmultiplied(dimensions, rgba.as_raw());
                    self.image_texture =
                        Some(context.load_texture("fundus", color, TextureOptions::LINEAR));
                    self.loaded_image_path = path;
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                }
                Err(error) => self.image_error = Some(format!("Could not open image: {error}")),
            }
        }
    }

    fn reports_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Medical reports");
        ui.label("Saved examination evidence. AI output remains empty when no verified model is available.");
        if !self.records.unlocked {
            ui.label("Open encrypted storage from Patients to view reports.");
            return;
        }
        egui::ComboBox::from_label("Examination")
            .selected_text(
                self.selected_examination
                    .map(|id| format!("Examination #{id}"))
                    .unwrap_or_else(|| "Select examination".into()),
            )
            .show_ui(ui, |ui| {
                for exam in &self.records.examinations {
                    ui.selectable_value(
                        &mut self.selected_examination,
                        exam.id,
                        format!(
                            "#{} • {} • {} eye",
                            exam.id.unwrap_or_default(),
                            exam.examined_at,
                            if exam.eye.is_empty() {
                                "unknown"
                            } else {
                                &exam.eye
                            }
                        ),
                    );
                }
            });
        let Some(exam) = self
            .records
            .examinations
            .iter()
            .find(|exam| exam.id == self.selected_examination)
        else {
            return;
        };
        let patient = self
            .records
            .patients
            .iter()
            .find(|patient| patient.id == exam.patient_id);
        ui.separator();
        ui.strong(
            patient
                .map(|patient| format!("{} — {}", patient.patient_code, patient.display_name))
                .unwrap_or_else(|| format!("Patient #{}", exam.patient_id)),
        );
        ui.label(format!(
            "Examined: {} • Eye: {} • Examiner: {}",
            exam.examined_at,
            if exam.eye.is_empty() {
                "not recorded"
            } else {
                &exam.eye
            },
            exam.operator
        ));
        ui.label(format!(
            "Image: {}",
            if exam.image_path.is_empty() {
                "not recorded"
            } else {
                &exam.image_path
            }
        ));
        ui.label(format!("Notes: {}", exam.notes));
        if exam.analysis_json.is_empty() {
            ui.colored_label(
                Color32::from_rgb(143, 91, 20),
                "No AI request was saved with this examination.",
            );
        } else if let Ok(analysis) = serde_json::from_str::<Value>(&exam.analysis_json) {
            ui.label(format!(
                "AI status: {}",
                analysis["status"].as_str().unwrap_or("unknown")
            ));
            ui.label(format!(
                "Result: {}",
                analysis["error"]["code"]
                    .as_str()
                    .unwrap_or("No clinical finding recorded")
            ));
            ui.label(format!("Measured timings: {}", analysis["timings_ms"]));
            ui.label(format!(
                "Findings: {}",
                analysis["data"]["findings"].as_array().map_or(0, Vec::len)
            ));
        }
        if ui.button("Export report JSON…").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Export RetinaCare report")
                .set_file_name("retinacare-report.json")
                .save_file()
        {
            let report = serde_json::json!({"patient": patient.map(|patient| serde_json::json!({"code":patient.patient_code,"name":patient.display_name})), "examination": {"id":exam.id,"examined_at":exam.examined_at,"eye":exam.eye,"operator":exam.operator,"notes":exam.notes,"image_path":exam.image_path}, "analysis": serde_json::from_str::<Value>(&exam.analysis_json).unwrap_or(Value::Null), "clinical_review_required":true});
            self.notice = Some(
                match std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap_or_default()) {
                    Ok(()) => "Report exported.".into(),
                    Err(error) => format!("Report export failed: {error}"),
                },
            );
        }
        ui.small("PDF export is not implemented; JSON export preserves the real saved data without fabricating report content.");
        if let Some(notice) = &self.notice {
            ui.label(notice);
        }
    }

    fn settings_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings and local runtime");
        match ResourcePaths::discover() {
            Ok(paths) => {
                ui.label(format!("Application data: {}", paths.data.display()));
                ui.label(format!("Python runtime: {}", paths.python.display()));
                ui.label(format!("AI worker: {}", paths.worker_source.display()));
                ui.label(format!("Model folder: {}", paths.models.display()));
                ui.small("Install only licensed, checksum-verified model weights. Restart RetinaCare after changing models.");
            }
            Err(error) => {
                ui.colored_label(Color32::LIGHT_RED, error.to_string());
            }
        }
        if ui.button("Refresh worker health").clicked() {
            self.worker.refresh();
        }
    }

    fn show_screen(&mut self, ui: &mut egui::Ui) {
        match self.screen {
            Screen::Dashboard => self.dashboard(ui),
            Screen::Patients => self.patient_screen(ui),
            Screen::Screening => self.screening_screen(ui),
            Screen::History => self.history_screen(ui),
            Screen::Results => {
                ui.heading("AI analysis results");
                ui.label(format!(
                    "Status: {}",
                    self.last_analysis["status"]
                        .as_str()
                        .unwrap_or("No analysis result available")
                ));
                ui.label(format!(
                    "Result: {}",
                    self.last_analysis["error"]["code"]
                        .as_str()
                        .unwrap_or("No clinical output")
                ));
                ui.label(format!(
                    "Measured timings: {}",
                    self.last_analysis["timings_ms"]
                ));
                ui.label(format!(
                    "Findings: {}",
                    self.last_analysis["data"]["findings"]
                        .as_array()
                        .map_or(0, Vec::len)
                ));
                ui.label("No diagnosis, lesion map, mask, or benchmark is synthesized.");
            }
            Screen::Reports => self.reports_screen(ui),
            Screen::Settings => self.settings_screen(ui),
            Screen::Gpu => {
                ui.heading("GPU performance");
                let device = &self.worker_status["data"]["device"];
                ui.label(format!(
                    "CUDA available: {}",
                    device["cuda_available"].as_bool().unwrap_or(false)
                ));
                ui.label(format!(
                    "Device: {}",
                    device["device_name"].as_str().unwrap_or("Unavailable")
                ));
                ui.label(format!("VRAM telemetry: {}", device["memory_bytes"]));
                ui.label("No performance benchmark has been run.");
            }
        }
    }
}

impl eframe::App for RetinaCareUi {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(status) = self.worker.take_latest() {
            if status["command"] == "analyze_image" {
                self.last_analysis = status;
            } else {
                self.worker_status = status;
            }
        }
        self.poll_image(ui.ctx());
        if let Some(update) = self.database.take_latest() {
            match update {
                Ok(records) => {
                    self.records = records;
                    self.database_error = None;
                }
                Err(error) => self.database_error = Some(error),
            }
        }
        if let Some((path, started, frames)) = &mut self.smoke {
            *frames += 1;
            let screens = [
                Screen::Dashboard,
                Screen::Patients,
                Screen::Screening,
                Screen::History,
                Screen::Results,
                Screen::Reports,
                Screen::Settings,
                Screen::Gpu,
            ];
            self.screen = screens[(*frames as usize / 10) % screens.len()];
            let worker_ok = self.worker_status["status"] == "ok";
            if (*frames >= 70 && worker_ok) || started.elapsed() > Duration::from_secs(15) {
                let report = serde_json::json!({"gui_frames":frames, "navigation_screens":8,"worker_health_seen":worker_ok,"passed":worker_ok && *frames >= 70});
                if let Err(error) = std::fs::write(path, report.to_string()) {
                    eprintln!("GUI smoke report failed: {error}");
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if let Some((path, frames, requested)) = &mut self.screenshot {
            *frames += 1;
            if *frames >= 12 && !*requested {
                *requested = true;
                let path = path.clone();
                let context = ui.ctx().clone();
                ui.ctx().request_screenshot(move |capture| {
                    let mut rgba = Vec::with_capacity(capture.pixels.len() * 4);
                    for pixel in &capture.pixels {
                        rgba.extend_from_slice(&pixel.to_array());
                    }
                    let _ = image::save_buffer(
                        path,
                        &rgba,
                        capture.width() as u32,
                        capture.height() as u32,
                        image::ColorType::Rgba8,
                    );
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                });
            }
        }
        let full_height = ui.available_height();
        ui.horizontal(|ui| {
            ui.set_min_height(full_height);
            ui.vertical(|ui| {
                ui.set_width(226.0);
                ui.set_min_height(full_height);
                self.navigation(ui);
            });
            egui::Frame::new()
                .fill(Color32::from_rgb(248, 249, 250))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 45, 54)))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::same(20))
                .show(ui, |ui| {
                    ui.set_min_height((full_height - 40.0).max(1.0));
                    ui.set_min_width(ui.available_width());
                    ui.vertical(|ui| self.show_screen(ui));
                });
        });
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(150));
    }
}

fn status_card(ui: &mut egui::Ui, title: &str, value: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(160.0);
        ui.strong(title);
        ui.label(value);
    });
}

fn device_name(status: &Value) -> &str {
    status["data"]["device"]["device_name"]
        .as_str()
        .unwrap_or("CPU / unavailable")
}

fn decode_image(path: &str) -> Result<image::RgbaImage, String> {
    let path = std::path::Path::new(path);
    let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 100 * 1024 * 1024 {
        return Err("Image must be a regular file below 100 MiB".into());
    }
    let dimensions = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_dimensions()
        .map_err(|e| e.to_string())?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > 40_000_000 {
        return Err("Image exceeds 40 megapixels".into());
    }
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|image| image.to_rgba8())
        .map_err(|e| e.to_string())
}

fn app_icon() -> Result<egui::IconData, image::ImageError> {
    let rgba =
        image::load_from_memory(include_bytes!("../../../../icon/RetinaCare.ico"))?.to_rgba8();
    Ok(egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--smoke-test") {
        let report = smoke_test();
        if let Some(path) = args.get(index + 1) {
            std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
        }
        println!("{report}");
        if report["passed"] != true {
            return Err("packaged smoke test failed".into());
        }
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(app_icon()?),
        ..Default::default()
    };
    eframe::run_native(
        "RetinaCare — AI Manthan PS14",
        options,
        Box::new(|context| Ok(Box::new(RetinaCareUi::new(context)))),
    )
    .map_err(Into::into)
}

fn smoke_test() -> Value {
    let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
        let paths = ResourcePaths::discover()?;
        let mut worker = retinacare_app::smoke_worker(&paths)?;
        let health = worker.request("health", serde_json::json!({}))?;
        if health["status"] != "ok" {
            return Err("worker health failed".into());
        }
        retinacare_app::smoke_database()?;
        let icon = app_icon()?;
        Ok(
            serde_json::json!({"passed":true,"worker_health":health,"encrypted_database":true,"icon_dimensions":[icon.width,icon.height]}),
        )
    })();
    result.unwrap_or_else(|error| serde_json::json!({"passed":false,"error":error.to_string()}))
}
