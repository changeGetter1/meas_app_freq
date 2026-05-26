# meas_app_freq

Agilent 53131A/132A 225 MHz ユニバーサルカウンタ向け GPIB/VISA リアルタイム計測アプリ

## 機能

- CH1 周波数をリアルタイムグラフ表示
- 測定モード・ゲート時間・計測周期をプリセットから選択（JSON で追加可能）
- プロット折れ線色をユーザー設定
- 計測結果を TSV ファイルに自動保存（`data/` フォルダ）
- 統計情報をリアルタイム表示（Count / Min / Avg / Max / Std）

## スクリーンショット

<!-- 画像を貼る -->

## 必要環境

- Windows 10 / 11
- [NI-VISA](https://www.ni.com/ja-jp/support/downloads/drivers/download.ni-visa.html) または [Keysight IO Libraries Suite](https://www.keysight.com/us/en/lib/software-detail/computer-software/io-libraries-suite-downloads-2175637.html)
- Rust stable toolchain (`rustup` 推奨)

## ビルド

```powershell
cargo build --release
```

実行ファイルは `target/release/meas_app_freq.exe` に生成されます。

## 実行

実行ファイルと同じフォルダに設定ファイルを配置してから起動してください。

```
meas_app_freq.exe
config_init.json
config_periods.json
```

## 設定ファイル

### `config_init.json` — 計測モードプリセット

SCPI コマンド列・換算係数・単位を定義します。  
プリセットを追加・変更する場合はこのファイルを編集するだけでよく、コードの変更は不要です。

```json
{
  "presets": [
    {
      "name": "Freq CH1 0.1s gate",
      "init_commands": [
        "*RST", "*CLS", "*SRE 0", "*ESE 0",
        ":STAT:PRES",
        ":FUNC 'FREQ 1'",
        ":FREQ:ARM:STAR:SOUR IMM",
        ":FREQ:ARM:STOP:SOUR TIM",
        ":FREQ:ARM:STOP:TIM 0.1"
      ],
      "measure_command": "READ:FREQ?",
      "scale": 1.0,
      "unit": "Hz"
    }
  ]
}
```

### `config_periods.json` — 計測周期プリセット

```json
{
  "presets": [
    { "name": "Continuous (~0.1s gate)", "period_ms": 150 },
    { "name": "1s", "period_ms": 1000 }
  ]
}
```

## 出力ファイル

計測開始ごとに `data/` フォルダへ同一タイムスタンプのファイルが2本生成されます。

| ファイル | 内容 |
|---|---|
| `data/setting_YYYYMMDD_HHMMSS.tsv` | 計測条件（アドレス・プリセット・コマンド等） |
| `data/freq_YYYYMMDD_HHMMSS.tsv` | 計測結果（timestamp / elapsed_s / value_raw_hz / value_scaled / unit） |

## ライセンス

MIT