use eframe::egui;
use usb_doctor::inventory::ThemeChoice;
pub fn apply(ctx: &egui::Context, theme: ThemeChoice) {
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(match theme {
        ThemeChoice::System => egui::SystemTheme::SystemDefault,
        ThemeChoice::Light => egui::SystemTheme::Light,
        ThemeChoice::Dark => egui::SystemTheme::Dark,
    }));
    ctx.set_theme(match theme {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Light => egui::ThemePreference::Light,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
    });
}
pub fn install(ctx: &egui::Context, theme: ThemeChoice) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "compact".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/RobotoCondensed.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "compact".into());
    ctx.set_fonts(fonts);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut s = (*ctx.style_of(theme)).clone();
        s.spacing.item_spacing = egui::vec2(7.0, 3.0);
        s.spacing.button_padding = egui::vec2(7.0, 3.0);
        s.spacing.scroll.floating = false;
        s.spacing.scroll.bar_width = 14.0;
        s.spacing.scroll.handle_min_length = 30.0;
        s.spacing.scroll.foreground_color = true;
        s.text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        s.text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        ctx.set_style_of(theme, s);
    }
    apply(ctx, theme);
}
