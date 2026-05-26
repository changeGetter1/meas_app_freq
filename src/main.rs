mod app;
mod measurement;
mod model;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 570.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Frequency Measurement - Agilent 53131A/132A",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            // 日本語フォントをフォールバックとして追加
            let mut fonts = egui::FontDefinitions::default();
            let font_paths = [
                r"C:\Windows\Fonts\YuGothR.ttc",
                r"C:\Windows\Fonts\meiryo.ttc",
                r"C:\Windows\Fonts\msgothic.ttc",
            ];
            for path in &font_paths {
                if let Ok(data) = std::fs::read(path) {
                    fonts.font_data.insert(
                        "ja_fallback".to_owned(),
                        std::sync::Arc::new(egui::FontData::from_owned(data)),
                    );
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .push("ja_fallback".to_owned());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push("ja_fallback".to_owned());
                    cc.egui_ctx.set_fonts(fonts);
                    break;
                }
            }

            Ok(Box::new(app::App::new()))
        }),
    )
}