use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use parking_lot::Mutex;
use visa_rs::prelude::*;

use crate::measurement::measurement_thread;
use crate::model::{InitConfig, PeriodConfig, RunState, SharedState};

pub struct App {
    visa_addresses: Vec<String>,
    selected_addr_idx: usize,

    period_cfg: PeriodConfig,
    init_cfg: InitConfig,
    selected_period_idx: usize,
    selected_init_idx: usize,

    plot_color: egui::Color32,

    status: String,

    shared: Arc<Mutex<SharedState>>,
}

impl App {
    pub fn new() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let period_cfg: PeriodConfig = {
            let path = exe_dir.join("config_periods.json");
            let f = File::open(&path)
                .with_context(|| format!("failed to open {:?}", path))
                .unwrap();
            serde_json::from_reader(f)
                .with_context(|| format!("failed to parse {:?}", path))
                .unwrap()
        };

        let init_cfg: InitConfig = {
            let path = exe_dir.join("config_init.json");
            let f = File::open(&path)
                .with_context(|| format!("failed to open {:?}", path))
                .unwrap();
            serde_json::from_reader(f)
                .with_context(|| format!("failed to parse {:?}", path))
                .unwrap()
        };

        let (visa_addresses, status) = match Self::scan_visa() {
            Ok(list) if !list.is_empty() => (list, "VISA scan OK".to_string()),
            Ok(_) => (vec![], "No VISA instruments found".to_string()),
            Err(e) => (vec![], format!("VISA scan error: {e:#}")),
        };

        App {
            visa_addresses,
            selected_addr_idx: 0,
            period_cfg,
            init_cfg,
            selected_period_idx: 0,
            selected_init_idx: 0,
            plot_color: egui::Color32::from_rgb(30, 144, 255),
            status,
            shared: Arc::new(Mutex::new(SharedState {
                run_state: RunState::Idle,
                should_stop: false,
                data: Vec::new(),
            })),
        }
    }

    fn scan_visa() -> Result<Vec<String>> {
        let rm = DefaultRM::new()?;
        let expr = std::ffi::CString::new("GPIB?*INSTR")?.into();
        let mut list = Vec::new();
        let res_list = rm.find_res_list(&expr)?;

        for r in res_list {
            list.push(r?.to_string_lossy().to_string());
        }
        Ok(list)
    }

    pub fn start_measurement(&mut self) {
        if self.visa_addresses.is_empty() {
            self.status = "No VISA address selected".into();
            return;
        }

        let addr = self.visa_addresses[self.selected_addr_idx].clone();
        let period = self.period_cfg.presets[self.selected_period_idx].period_ms;
        let init_preset = self.init_cfg.presets[self.selected_init_idx].clone();

        {
            let mut s = self.shared.lock();
            if s.run_state == RunState::Running {
                self.status = "Already running".into();
                return;
            }
            s.should_stop = false;
            s.data.clear();
            s.run_state = RunState::Idle;
        }

        let shared = self.shared.clone();
        self.status = "Starting...".into();

        thread::spawn(move || {
            if let Err(e) = measurement_thread(shared, addr, period, init_preset) {
                eprintln!("measurement thread error: {e:#}");
            }
        });
    }

    pub fn stop_measurement(&mut self) {
        let mut s = self.shared.lock();
        if s.run_state == RunState::Running {
            s.should_stop = true;
            self.status = "Stopping...".into();
        }
    }

    pub fn restart_measurement(&mut self) {
        let mut s = self.shared.lock();
        if s.run_state == RunState::Initialized {
            s.should_stop = false;
            s.run_state = RunState::Running;
            self.status = "Restart requested".into();
        } else {
            self.status = "Cannot restart: not in Initialized state".into();
        }
    }

    pub fn clear_all(&mut self) {
        let mut s = self.shared.lock();
        s.should_stop = true;
        s.data.clear();
        s.run_state = RunState::Idle;
        self.status = "Cleared".into();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        let shared_snapshot = {
            let s = self.shared.lock();
            (s.run_state, s.data.clone())
        };

        let current_unit = self.init_cfg.presets
            .get(self.selected_init_idx)
            .map(|p| p.unit.clone())
            .unwrap_or_else(|| "?".into());

        // 統計計算（パネル間で共有）
        let valid_values: Vec<f64> = shared_snapshot
            .1
            .iter()
            .map(|p| p.value_scaled)
            .filter(|v| v.is_finite())
            .collect();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Frequency Measurement (Agilent 53131A/132A)");
            ui.label(&self.status);
        });

        egui::TopBottomPanel::bottom("stats_panel").show(ctx, |ui| {
            ui.separator();
            if valid_values.is_empty() {
                ui.label("Statistics: ---");
            } else {
                let count = valid_values.len();
                let min = valid_values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = valid_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let mean = valid_values.iter().sum::<f64>() / count as f64;
                let variance = valid_values
                    .iter()
                    .map(|v| (v - mean).powi(2))
                    .sum::<f64>()
                    / count as f64;
                let stddev = variance.sqrt();

                ui.horizontal(|ui| {
                    ui.label(format!("Count: {count}"));
                    ui.separator();
                    ui.label(format!("Min: {min:.6} {current_unit}"));
                    ui.separator();
                    ui.label(format!("Avg: {mean:.6} {current_unit}"));
                    ui.separator();
                    ui.label(format!("Max: {max:.6} {current_unit}"));
                    ui.separator();
                    ui.label(format!("Std: {stddev:.6} {current_unit}"));
                });
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // 設定行: アドレス・測定モード・周期を1行に集約
            ui.horizontal(|ui| {
                ui.label("Addr:");
                let addr_text = self.visa_addresses
                    .get(self.selected_addr_idx)
                    .cloned()
                    .unwrap_or_else(|| "No instruments".into());
                egui::ComboBox::from_id_salt("visa_addr")
                    .width(160.0)
                    .selected_text(addr_text)
                    .show_ui(ui, |ui| {
                        if self.visa_addresses.is_empty() {
                            ui.label("No instruments");
                        } else {
                            for (i, addr) in self.visa_addresses.iter().enumerate() {
                                if ui
                                    .selectable_label(i == self.selected_addr_idx, addr)
                                    .clicked()
                                {
                                    self.selected_addr_idx = i;
                                }
                            }
                        }
                    });
                if ui.button("Rescan").clicked() {
                    match Self::scan_visa() {
                        Ok(list) if !list.is_empty() => {
                            self.visa_addresses = list;
                            self.selected_addr_idx = 0;
                            self.status = "Rescan OK".into();
                        }
                        Ok(_) => {
                            self.visa_addresses.clear();
                            self.status = "No VISA instruments found".into();
                        }
                        Err(e) => {
                            self.status = format!("Rescan error: {e:#}");
                        }
                    }
                }

                ui.separator();

                ui.label("Mode:");
                egui::ComboBox::from_id_salt("init_preset")
                    .width(200.0)
                    .selected_text(
                        self.init_cfg
                            .presets
                            .get(self.selected_init_idx)
                            .map(|p| format!("{} [{}]", p.name, p.unit))
                            .unwrap_or_else(|| "N/A".into()),
                    )
                    .show_ui(ui, |ui| {
                        for (i, p) in self.init_cfg.presets.iter().enumerate() {
                            let label = format!("{} [{}]", p.name, p.unit);
                            if ui
                                .selectable_label(i == self.selected_init_idx, label)
                                .clicked()
                            {
                                self.selected_init_idx = i;
                            }
                        }
                    });

                ui.separator();

                ui.label("Period:");
                egui::ComboBox::from_id_salt("period_preset")
                    .width(130.0)
                    .selected_text(
                        self.period_cfg
                            .presets
                            .get(self.selected_period_idx)
                            .map(|p| format!("{} ({}ms)", p.name, p.period_ms))
                            .unwrap_or_else(|| "N/A".into()),
                    )
                    .show_ui(ui, |ui| {
                        for (i, p) in self.period_cfg.presets.iter().enumerate() {
                            let label = format!("{} ({}ms)", p.name, p.period_ms);
                            if ui
                                .selectable_label(i == self.selected_period_idx, label)
                                .clicked()
                            {
                                self.selected_period_idx = i;
                            }
                        }
                    });

                ui.separator();

                ui.label("Line:");
                egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut self.plot_color,
                    egui::color_picker::Alpha::Opaque,
                );
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    self.start_measurement();
                }
                if ui.button("Stop").clicked() {
                    self.stop_measurement();
                }
                if ui.button("Restart").clicked() {
                    self.restart_measurement();
                }
                if ui.button("Clear").clicked() {
                    self.clear_all();
                }

                ui.label(format!("State: {:?}", shared_snapshot.0));
                ui.label(format!("Points: {}", shared_snapshot.1.len()));
            });

            // 最新測定値の表示
            if let Some(last) = shared_snapshot.1.last() {
                if last.value_scaled.is_finite() {
                    ui.label(
                        egui::RichText::new(format!(
                            "Latest: {:.6} {}",
                            last.value_scaled, current_unit
                        ))
                        .size(20.0),
                    );
                } else {
                    ui.label(egui::RichText::new("Latest: --- (invalid)").size(20.0));
                }
            }

            ui.separator();

            // NaN を除外してプロット
            let points: PlotPoints = shared_snapshot
                .1
                .iter()
                .enumerate()
                .filter(|(_, p)| p.value_scaled.is_finite())
                .map(|(i, p)| [i as f64, p.value_scaled])
                .collect();

            Plot::new("plot")
                .height(ui.available_height())
                .y_axis_label(current_unit.as_str())
                .x_axis_label("経過時間[sec]")
                .show(ui, |plot_ui| {
                    plot_ui.line(Line::new(current_unit.as_str(), points).color(self.plot_color));
                });
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
