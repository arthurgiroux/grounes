#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use grounes::cpu::{StatusRegister, StepResult};
use grounes::emulator::Emulator;

use eframe::egui;

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
        }
    }
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
        egui::CentralPanel::default().show(ctx, |ui| {
            // Section 1: ROM loading
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
            });
            if let Some(err) = &self.load_error {
                ui.colored_label(egui::Color32::RED, err);
            }

            if !self.rom_loaded {
                return;
            }

            ui.separator();

            // Section 2: CPU state
            let cpu = &self.emulator.cpu;
            ui.monospace(format!(
                "PC: ${:04X}   A: ${:02X}   X: ${:02X}   Y: ${:02X}   SP: ${:02X}",
                cpu.pc, cpu.a, cpu.x, cpu.y, cpu.sp.value
            ));
            let p = cpu.p;
            ui.monospace(format!("Flags: N V U B D I Z C"));
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

            ui.separator();

            // Section 3: Step controls
            ui.horizontal(|ui| {
                if ui.button("Step 1").clicked() {
                    let result = self.emulator.step();
                    let n = self.instruction_history.len() + 1;
                    self.instruction_history
                        .push(format!("#{n}: {}", format_step(&result)));
                }
                ui.label("Steps:");
                ui.add(egui::TextEdit::singleline(&mut self.step_count).desired_width(50.0));
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

            ui.separator();

            // Section 4: Instruction history
            ui.label("Instruction history:");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in self.instruction_history.iter().rev() {
                    ui.monospace(entry);
                }
            });
        });
    }
}
