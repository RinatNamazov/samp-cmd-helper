/*****************************************************************************
 *
 *  PROJECT:        samp-cmd-helper
 *  LICENSE:        See LICENSE in the top level directory
 *  FILE:           gui.rs
 *  DESCRIPTION:    User interface
 *  COPYRIGHT:      (c) 2024 RINWARES <rinwares.com>
 *  AUTHOR:         Rinat Namazov <rinat.namazov@rinwares.com>
 *
 *****************************************************************************/

use crate::cmd_storage::CMD_PREFIX;
use crate::plugin::Plugin;
use crate::{gta, samp};
use egui::{
    epaint::Shadow, Color32, FontData, FontDefinitions, FontFamily, FontId, FontTweak, Key, Label,
    RichText, Rounding, Sense, TextStyle,
};
use std::ffi::CStr;

pub struct Ui {
    cmds_height: f32
}

impl Ui {
    pub fn new() -> Self {
        Self { cmds_height: 64.0 }
    }

    pub fn init_style(ctx: &egui::Context) {
        Self::setup_custom_fonts(ctx);
        Self::configure_text_styles(ctx);
        Self::configure_visuals(ctx);
    }

    fn add_font(fonts: &mut FontDefinitions, name: &str, font: &'static [u8]) {
        let name = name.to_string();
        let tweak = FontTweak::default();
        fonts
            .font_data
            .insert(name.clone(), FontData::from_static(font).tweak(tweak));
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, name.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push(name);
    }

    fn setup_custom_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        Self::add_font(
            &mut fonts,
            "Segoe UI Bold",
            include_bytes!("C:\\Windows\\Fonts\\segoeuib.ttf"),
        );
        ctx.set_fonts(fonts);
    }

    fn configure_text_styles(ctx: &egui::Context) {
        use FontFamily::{Monospace, Proportional};

        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, Proportional)),
            (TextStyle::Body, FontId::new(16.5, Proportional)),
            (TextStyle::Monospace, FontId::new(16.0, Monospace)),
            (TextStyle::Button, FontId::new(16.5, Proportional)),
            (TextStyle::Small, FontId::new(8.0, Proportional)),
        ]
        .into();
        ctx.set_style(style);
    }

    fn configure_visuals(ctx: &egui::Context) {
        let mut visuals = ctx.style().visuals.clone();
        visuals.window_shadow = Shadow::NONE;
        visuals.window_fill = Color32::from_rgba_premultiplied(20, 20, 20, 200);
        visuals.window_rounding = Rounding::same(10.);
        ctx.set_visuals(visuals);
    }

    pub fn render_ui(ctx: &egui::Context, this: &mut Ui) {
        if gta::is_gta_menu_active() {
            return;
        }

        // SA-MP keys.
        // Todo: It might be better to read the key from memory, in case there is a plugin to change the keys.
        if ctx.input(|i| i.key_down(Key::F5) || i.key_down(Key::F10)) {
            return;
        }

        let samp_input = match samp::Input::get() {
            Some(v) => v,
            None => return,
        };

        // Draw only if chat input is open.
        if !samp_input.enabled.as_bool() {
            return;
        }

        let chat_input = samp_input.edit_box().get_text();
        let chat_contains_cmd = chat_input.starts_with(CMD_PREFIX);

        // Don't draw empty list.
        if (samp_input.total_recall == 0 && !chat_contains_cmd)
            || (chat_contains_cmd && Plugin::get().commands().is_empty())
        {
            return;
        }

        let pos = samp_input.edit_box().position;
        let pos = [
            pos[0] as f32,
            (pos[1] + samp_input.edit_box().height + 5) as f32,
        ];

        // So that each window has its own size.
        let key = if chat_contains_cmd {
            "#Commands"
        } else {
            "#Recalls"
        };
        egui::containers::Window::new(key)
            .fixed_pos(pos)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if chat_contains_cmd {
                    this.draw_commands(ui, &chat_input, samp_input)
                } else {
                    this.draw_recalls(ui, samp_input);
                }
                this.draw_copyright(ui);
            });
    }

    fn draw_commands(&mut self, ui: &mut egui::Ui, chat_input: &String, samp_input: &mut samp::Input) {
        egui::Grid::new("cmds").min_col_width(200.0).show(ui, |ui| {
            self.draw_cmds_header(ui);
            ui.end_row();
            self.draw_cmds_body(ui, &chat_input, samp_input);
            ui.end_row();
        });
    }

    fn draw_cmds_header(&self, ui: &mut egui::Ui) {
        for category in Plugin::get().commands().iter() {
            if category.is_visible {
                ui.vertical_centered(|ui| {
                    ui.strong(&category.name);
                });
            }
        }
    }

    fn draw_cmds_body(&mut self, ui: &mut egui::Ui, chat_input: &String, input: &mut samp::Input) {
        let cursor_top = ui.cursor().top();
        let mut max_content_height = 0.;

        for category in Plugin::get().commands().iter() {
            if !category.is_visible {
                continue;
            }

            let content_height = egui::ScrollArea::vertical()
                .id_source(&category.name)
                .min_scrolled_height(self.cmds_height)
                .show(ui, |ui| {
                ui.vertical(|ui| {
                    for (name, commands) in category.modules.iter() {
                        egui::CollapsingHeader::new(name)
                            .default_open(true)
                            .show(ui, |ui| {
                                for (cmd, description) in commands.iter() {
                                    let text = if chat_input.is_empty() || cmd.starts_with(chat_input) {
                                        RichText::new(cmd)
                                    } else {
                                        RichText::new(cmd).weak()
                                    };

                                    let label = ui.add(Label::new(text).sense(Sense::click()));

                                    if label.clicked() {
                                        input.edit_box().set_text(cmd.as_str());
                                    }

                                    if !description.is_empty() {
                                        label.on_hover_text(description);
                                    }
                                }
                            });
                    }
                });
            }).content_size.y;

            if content_height > max_content_height {
                max_content_height = content_height;
            }
        }

        let max_screen_height = ui.input(|i| i.screen_rect.height()) - cursor_top - 100.;
        self.cmds_height = max_content_height.min(max_screen_height);
    }

    fn draw_copyright(&self, ui: &mut egui::Ui) {
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.strong("Copyright © Rinat Namazov").on_hover_ui(|ui| {
                ui.label(concat!("SA-MP Command Helper v", env!("CARGO_PKG_VERSION")));
                ui.label("https://rinwares.com");
            });
        });
    }

    fn draw_recalls(&self, ui: &mut egui::Ui, input: &mut samp::Input) {
        ui.vertical_centered(|ui| {
            ui.strong("Recalls");
        });

        ui.indent(ui.id(), |ui| {
            for i in 0..input.total_recall as usize {
                if let Ok(recall) = CStr::from_bytes_until_nul(&input.recall_buffer[i]) {
                    if let Ok(text) = recall.to_str() {
                        let text =
                            if input.current_recall == -1 || i == input.current_recall as usize {
                                RichText::new(text)
                            } else {
                                RichText::new(text).weak()
                            };

                        let label = ui.add(Label::new(text).sense(Sense::click()));

                        if label.clicked() {
                            input.current_recall = i as i32;
                            input.edit_box().set_text_raw(recall.as_ptr());
                        }
                    }
                }
            }
        });
    }
}
