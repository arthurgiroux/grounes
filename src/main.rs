#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use grounes::emulator::{self, Emulator};
use std::io;

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
}

impl GrounesApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        GrounesApp {
            rom_loaded: false,
            emulator: Emulator::new(),
        }
    }
}

impl eframe::App for GrounesApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.rom_loaded {
                if ui.button("Load rom").clicked() {
                    let res = self.emulator.load_rom("data/nestest.nes");
                    if res.is_ok() {
                        self.emulator.power_up();
                        self.rom_loaded = true
                    }
                }
            } else {
                if ui.button("Step CPU").clicked() {
                    self.emulator.step();
                }
                ui.heading(format!("{:?}", self.emulator.cpu));
            }
        });
    }
}
