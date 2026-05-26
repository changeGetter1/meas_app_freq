use chrono::{DateTime, Local};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct PeriodPreset {
    pub name: String,
    pub period_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PeriodConfig {
    pub presets: Vec<PeriodPreset>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InitPreset {
    pub name: String,
    pub init_commands: Vec<String>,
    pub measure_command: String,
    pub scale: f64,
    pub unit: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InitConfig {
    pub presets: Vec<InitPreset>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct MeasurementPoint {
    pub t: DateTime<Local>,
    pub value_raw: f64,
    pub value_scaled: f64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RunState {
    Idle,
    Initialized,
    Running,
}

pub struct SharedState {
    pub run_state: RunState,
    pub should_stop: bool,
    pub data: Vec<MeasurementPoint>,
}
