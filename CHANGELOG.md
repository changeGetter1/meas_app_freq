# Changelog

## [0.1.0] - 2026-05-27

### 初版リリース

- Agilent 53131A/132A CH1 周波数計測（ゲート時間 0.1 s、`READ:FREQ?`）
- GPIB/VISA 接続・アドレス自動スキャン（`GPIB?*INSTR` フィルタ）
- リアルタイムグラフ表示（egui_plot）
- ダークテーマ UI
- プロット折れ線色をユーザー設定（カラーピッカー）
- 統計情報リアルタイム表示（Count / Min / Avg / Max / Std）
- 計測結果を TSV ファイルへ自動保存（`data/` フォルダ）
  - `setting_YYYYMMDD_HHMMSS.tsv`：計測条件（開始時 1 回）
  - `freq_YYYYMMDD_HHMMSS.tsv`：計測結果（timestamp / elapsed_s / value_raw_hz / value_scaled / unit）
- 計測モード・周期を JSON プリセットで設定（`config_init.json` / `config_periods.json`）
- 9.91E+37 センチネル値を NaN として処理