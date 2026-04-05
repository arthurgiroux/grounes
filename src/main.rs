#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use egui_memory_editor::MemoryEditor;
use grounes::cpu::{StatusRegister, StepResult};
use grounes::emulator::Emulator;

use eframe::egui::{self, Color32, Layout};
use grounes::memory::MemoryBus;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Grounes",
        native_options,
        Box::new(|cc| Ok(Box::new(GrounesApp::new(cc)))),
    )
}

struct GrounesApp {
    rom_loaded: bool,
    emulator: Emulator,
    rom_path: String,
    step_count: String,
    instruction_history: Vec<String>,
    load_error: Option<String>,
    show_debug_panel: bool,
    step_by_step: bool,
    memory_editor: MemoryEditor,
    chr_texture: Option<egui::TextureHandle>,
    frame_texture: Option<egui::TextureHandle>,
}

impl GrounesApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        GrounesApp {
            rom_loaded: false,
            emulator: Emulator::new(),
            rom_path: "data/nestest.nes".to_string(),
            step_count: "1".to_string(),
            instruction_history: vec![],
            load_error: None,
            show_debug_panel: false,
            step_by_step: false,
            memory_editor: MemoryEditor::new()
                .with_address_range("All", 0..0xFFFF)
                .with_address_range("RAM", 0..0x1FFF)
                .with_address_range("PPU", 0x2000..0x4000)
                .with_address_range("APU", 0x4000..0x401F)
                .with_address_range("Cartridge", 0x4020..0xFFFF),
            chr_texture: None,
            frame_texture: None,
        }
    }
}

fn decode_pattern_table(chr: &[u8]) -> egui::ColorImage {
    const W: usize = 128;
    const H: usize = 128;
    let mut pixels = vec![egui::Color32::BLACK; W * H];

    // Pattern table is a 16x16 table of tiles
    for tile in 0..256usize {
        let tile_x = (tile % 16) * 8;
        let tile_y = (tile / 16) * 8;
        let base = tile * 16;

        for row in 0..8usize {
            let lo = chr.get(base + row).copied().unwrap_or(0);
            let hi = chr.get(base + row + 8).copied().unwrap_or(0);
            for col in 0..8usize {
                let bit = 7 - col;
                let value = ((lo >> bit) & 1) | (((hi >> bit) & 1) << 1);
                let color = match value {
                    0 => egui::Color32::BLACK,
                    1 => egui::Color32::DARK_GRAY,
                    2 => egui::Color32::LIGHT_GRAY,
                    _ => egui::Color32::WHITE,
                };
                pixels[(tile_y + row) * W + (tile_x + col)] = color;
            }
        }
    }

    egui::ColorImage::new([W, H], pixels)
}

fn format_step(result: &StepResult) -> String {
    match &result.opcode {
        Some(op) => format!(
            "{:?} ({:?}) | ${:02X} | {} cycles",
            op.instr, op.mode, op.value, result.cycles
        ),
        None => "Invalid opcode".to_string(),
    }
}

fn flag_char(p: StatusRegister, flag: StatusRegister) -> &'static str {
    if p.contains(flag) { "1" } else { "0" }
}

impl eframe::App for GrounesApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.rom_loaded && !self.step_by_step {
            self.emulator.step_frame();
            ctx.request_repaint();
        }

        // Top toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("ROM:");
                ui.text_edit_singleline(&mut self.rom_path);
                if ui.button("Load").clicked() {
                    match self.emulator.load_rom(&self.rom_path) {
                        Ok(()) => {
                            self.emulator.power_up();
                            self.rom_loaded = true;
                            self.instruction_history.clear();
                            self.load_error = None;
                        }
                        Err(e) => {
                            self.load_error = Some(e.to_string());
                        }
                    }
                }
                if let Some(err) = &self.load_error {
                    ui.colored_label(Color32::RED, err);
                }
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.toggle_value(&mut self.show_debug_panel, "Debug");
                    ui.checkbox(&mut self.step_by_step, "Step-by-step");
                });
            });
        });

        // Right debug panel
        if self.show_debug_panel {
            egui::SidePanel::right("debug_panel")
                .resizable(true)
                .min_width(300.0)
                .show(ctx, |ui| {
                    if !self.rom_loaded {
                        ui.label("Load a ROM to see debug info.");
                        return;
                    }

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // CPU State
                        egui::CollapsingHeader::new("CPU State")
                            .default_open(true)
                            .show(ui, |ui| {
                                let cpu = &self.emulator.cpu;
                                ui.monospace(format!(
                                    "PC: ${:04X}   A: ${:02X}   X: ${:02X}   Y: ${:02X}   SP: ${:02X}",
                                    cpu.pc, cpu.a, cpu.x, cpu.y, cpu.sp.value
                                ));
                                let p = cpu.p;
                                ui.monospace("Flags: N V U B D I Z C");
                                ui.monospace(format!(
                                    "       {} {} {} {} {} {} {} {}",
                                    flag_char(p, StatusRegister::Negative),
                                    flag_char(p, StatusRegister::Overflow),
                                    flag_char(p, StatusRegister::Unused),
                                    flag_char(p, StatusRegister::Break),
                                    flag_char(p, StatusRegister::Decimal),
                                    flag_char(p, StatusRegister::InterruptDisabled),
                                    flag_char(p, StatusRegister::Zero),
                                    flag_char(p, StatusRegister::Carry),
                                ));
                            });

                        // Step Controls
                        egui::CollapsingHeader::new("Step Controls")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("Step 1").clicked() {
                                        let result = self.emulator.step();
                                        let n = self.instruction_history.len() + 1;
                                        self.instruction_history
                                            .push(format!("#{n}: {}", format_step(&result)));
                                    }
                                    ui.label("Steps:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.step_count)
                                            .desired_width(50.0),
                                    );
                                    if ui.button("Step N").clicked() {
                                        if let Ok(n) = self.step_count.trim().parse::<usize>() {
                                            for _ in 0..n {
                                                let result = self.emulator.step();
                                                let idx = self.instruction_history.len() + 1;
                                                self.instruction_history
                                                    .push(format!("#{idx}: {}", format_step(&result)));
                                            }
                                        }
                                    }
                                });
                            });

                        // Instruction History
                        egui::CollapsingHeader::new("Instruction History")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("history")
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        for entry in self.instruction_history.iter().rev() {
                                            ui.monospace(entry);
                                        }
                                    });
                            });

                        // Pattern Tables
                        egui::CollapsingHeader::new("Pattern Tables")
                            .default_open(true)
                            .show(ui, |ui| {
                                if let Some(chr) = self.emulator.chr_rom() {
                                    let image = decode_pattern_table(chr);
                                    let texture = self.chr_texture.get_or_insert_with(|| {
                                        ctx.load_texture(
                                            "chr_pattern_table",
                                            image.clone(),
                                            egui::TextureOptions::NEAREST,
                                        )
                                    });
                                    texture.set(image, egui::TextureOptions::NEAREST);
                                    ui.add(
                                        egui::Image::new(&*texture)
                                            .fit_to_exact_size(egui::vec2(256.0, 256.0)),
                                    );
                                } else {
                                    ui.label("No CHR ROM");
                                }
                            });

                        // Memory View
                        egui::CollapsingHeader::new("Memory View")
                            .default_open(true)
                            .show(ui, |ui| {
                                self.memory_editor.draw_editor_contents(
                                    ui,
                                    &mut self.emulator.get_bus_view(),
                                    |mem, address| mem.read_byte(address as u16).into(),
                                    |mem, address, val| mem.write_byte(address as u16, val),
                                );
                            });
                    });
                });
        }

        // Central output area
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(rect, 0.0, Color32::BLACK);

            if let Some(frame_data) = &self.emulator.current_frame {
                use grounes::ppu::PPU;
                let pixels: Vec<Color32> = frame_data
                    .chunks_exact(PPU::IMG_BPP)
                    .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
                    .collect();
                let image = egui::ColorImage::new([PPU::IMG_WIDTH, PPU::IMG_HEIGHT], pixels);
                let texture = self.frame_texture.get_or_insert_with(|| {
                    ctx.load_texture("nes_frame", image.clone(), egui::TextureOptions::NEAREST)
                });
                texture.set(image, egui::TextureOptions::NEAREST);

                let available = ui.available_size();
                let scale = (available.x / PPU::IMG_WIDTH as f32)
                    .min(available.y / PPU::IMG_HEIGHT as f32)
                    .floor()
                    .max(1.0);
                let size = egui::vec2(
                    PPU::IMG_WIDTH as f32 * scale,
                    PPU::IMG_HEIGHT as f32 * scale,
                );
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::new(&*texture).fit_to_exact_size(size));
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(Color32::DARK_GRAY, "Screen output");
                });
            }
        });
    }
}
