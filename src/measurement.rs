use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Local;
use parking_lot::Mutex;
use visa_rs::prelude::*;

use crate::model::{InitPreset, MeasurementPoint, RunState, SharedState};

pub fn measurement_thread(
    shared: Arc<Mutex<SharedState>>,
    addr: String,
    period_ms: u64,
    init_preset: InitPreset,
) -> Result<()> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let data_dir = exe_dir.join("data");
    std::fs::create_dir_all(&data_dir)?;

    let start_time = Local::now();
    let ts = start_time.format("%Y%m%d_%H%M%S").to_string();

    // 測定条件ファイル（開始時に1回書き出し）
    let setting_path = data_dir.join(format!("setting_{ts}.tsv"));
    let mut setting_writer = BufWriter::new(File::create(&setting_path)?);
    writeln!(setting_writer, "key\tvalue")?;
    writeln!(setting_writer, "start_time\t{}", start_time.to_rfc3339())?;
    writeln!(setting_writer, "addr\t{addr}")?;
    writeln!(setting_writer, "preset_name\t{}", init_preset.name)?;
    writeln!(setting_writer, "period_ms\t{period_ms}")?;
    writeln!(setting_writer, "measure_command\t{}", init_preset.measure_command)?;
    writeln!(setting_writer, "scale\t{}", init_preset.scale)?;
    writeln!(setting_writer, "unit\t{}", init_preset.unit)?;
    for (i, cmd) in init_preset.init_commands.iter().enumerate() {
        writeln!(setting_writer, "init_command_{i}\t{cmd}")?;
    }
    setting_writer.flush()?;

    // 測定結果ファイル
    let freq_path = data_dir.join(format!("freq_{ts}.tsv"));
    let mut writer = BufWriter::new(File::create(&freq_path)?);
    writeln!(writer, "timestamp\telapsed_s\tvalue_raw_hz\tvalue_scaled\tunit")?;

    let rm = DefaultRM::new()?;
    let addr_c = std::ffi::CString::new(addr.clone())?.into();
    let instr = rm.open(&addr_c, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;

    for cmd in &init_preset.init_commands {
        (&instr)
            .write_all(format!("{cmd}\n").as_bytes())
            .map_err(io_to_vs_err)?;
    }

    {
        let mut s = shared.lock();
        s.run_state = RunState::Initialized;
    }

    let period = Duration::from_millis(period_ms);
    let mut next = Instant::now();

    loop {
        {
            let mut s = shared.lock();
            if s.should_stop {
                s.run_state = RunState::Initialized;
                break;
            }
            s.run_state = RunState::Running;
        }

        next += period;
        let now = Local::now();

        (&instr)
            .write_all(format!("{}\n", init_preset.measure_command).as_bytes())
            .map_err(io_to_vs_err)?;

        let mut buf_reader = std::io::BufReader::new(&instr);
        let mut line = String::new();
        buf_reader.read_line(&mut line).map_err(io_to_vs_err)?;
        let raw: f64 = {
            let v: f64 = line.trim().parse().unwrap_or(f64::NAN);
            if v > 9.9e36 { f64::NAN } else { v }
        };
        let scaled = raw * init_preset.scale;

        let point = MeasurementPoint {
            t: now,
            value_raw: raw,
            value_scaled: scaled,
        };

        {
            let mut s = shared.lock();
            s.data.push(point.clone());
        }

        let elapsed_s = (now - start_time).num_milliseconds() as f64 / 1000.0;
        writeln!(
            writer,
            "{}\t{:.3}\t{}\t{}\t{}",
            now.to_rfc3339(),
            elapsed_s,
            raw,
            scaled,
            init_preset.unit,
        )?;
        writer.flush()?;

        let now_i = Instant::now();
        if next > now_i {
            thread::sleep(next - now_i);
        } else {
            next = now_i;
        }
    }

    Ok(())
}