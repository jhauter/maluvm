use std::{collections::HashMap, io::Cursor};

use egui::ScrollArea;
use vm::{
    asm::{self, RawOp, DATA_START},
    interpreter::{self, Interpreter, InterpreterErrorType, SyscallHandler}, parse::{try_parse_ops_from_bytecode, MaybeRawOp},
};

use crate::code::{self, select_label, show_mem_op, value_table, Editor};

pub struct CompiledCode {
    pub interpreter: Interpreter,
    pub labels: Box<[(String, u32)]>,
    pub results: Vec<u32>,
    pub ops: Vec<(MaybeRawOp, u32)>,
}
pub enum AppError {
    InterpreterError(InterpreterErrorType),
    LabelDoesNotExist(String),
}
impl From<InterpreterErrorType> for AppError {
    fn from(value: InterpreterErrorType) -> Self {
        AppError::InterpreterError(value)
    }
}

pub struct Env {
    log: String,
}
impl Default for Env {
    fn default() -> Self {
        Self { log: Default::default() }
    }
}
pub struct TemplateApp {
    // Example stuff:
    label: String,
    value: f32,
    editor: Editor,
    code: Option<CompiledCode>,
    label_menu: bool,
    selected_label: Option<usize>,
    jump_dest: Option<usize>,

    selected_global_slot_slider: usize,
    selected_global_slot: Option<usize>,

    selected_local_slot_slider: usize,
    selected_local_slot: Option<usize>,
    env: Env, 

}
#[allow(non_upper_case_globals)]
pub mod syscall {
    pub const PrintDebugString: u32  = 0x00;   
}

#[derive(Debug, Copy, Clone)]
pub enum EnvError {
    InvalidMemAddr = 1,
    InvalidStringData = 2, 

    Unknown = 99,
}
impl EnvError {
    pub fn as_return_code(result: Result<(), EnvError>) -> u32 {
        match result {
            Ok(_) => 0,
            Err(e) => e as u32,
        } 
    }
}
impl From<InterpreterErrorType> for EnvError {
    fn from(value: InterpreterErrorType) -> Self {
        match value {
            InterpreterErrorType::InvalidStringData(_) => Self::InvalidStringData,
            InterpreterErrorType::AddrOutOfBounds(_) => Self::InvalidMemAddr,
            _ => EnvError::Unknown
        }
    }
}
impl Env {
    fn print_debug_string(&mut self, interpreter: &mut Interpreter, addr: u32, len: u32) -> Result<(), EnvError> {
        let string_data = interpreter.read_str(addr, len)?;
        self.log.push_str(string_data);
        Ok(())
    }
}
impl SyscallHandler for Env {
    fn on_syscall(&mut self, interpreter: &mut Interpreter, id: u32, args: &[u32]) -> u32 {
        match id {
            syscall::PrintDebugString => {
                let addr = args[0];
                let len = args[1];

                EnvError::as_return_code(self.print_debug_string(interpreter, addr, len))
            }
            _ => 0  
        }
    }
}
impl TemplateApp {
    fn parse_ops(&mut self) -> Result<(), std::io::Error> {
        //TODO: Das ist schreklich
        if let Some(code) = &mut self.code {
            let mut reader = Cursor::new(code.interpreter.inital_bytecode()); 
            let mut current_offset = DATA_START; 
            for op in try_parse_ops_from_bytecode(&mut reader) {
                let op = op?;
                code.ops.push((op.clone(), current_offset));
                current_offset += match &op {
                    MaybeRawOp::Op(raw_op) => raw_op.size_bytes() as u32,
                    MaybeRawOp::Unknown(_) => 1,
                };
            };
        };
        Ok(())
    }

    fn compile(&mut self) -> Result<(), InterpreterErrorType> {
        //TODO: Error Handling
        let text = &self.editor.code;
        let bytecode = asm::Parser::parse(&text).unwrap();
        self.selected_label = None;

        match self.code {
            Some(ref mut code) => {
                code.interpreter.reset_all(&bytecode.code).unwrap();
                code.labels = bytecode.labels;
                Ok(())
            }
            None => {
                let interpreter = Interpreter::from_bytecode(&bytecode.code)?;
                let code = CompiledCode {
                    interpreter,
                    labels: bytecode.labels,
                    results: Vec::new(),
                    ops: Vec::new()
                };
                self.code = Some(code);
                self.parse_ops()?;
                Ok(())
            }
        }
    }

    fn compile_run(&mut self) -> Result<(), InterpreterErrorType> {
        self.compile()?;
        let code = self.code.as_mut().unwrap();
        code.interpreter.run(&mut self.env).unwrap().clone_into(&mut code.results);
        
        Ok(())
    }
}
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            editor: Default::default(),
            code: None,
            label_menu: false,
            selected_label: None,
            jump_dest: None,
            selected_global_slot_slider: 0,
            selected_global_slot: None,
            selected_local_slot_slider: 0, 
            selected_local_slot: None,
            env: Default::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //TODO: (joh): Nutze Arena hier

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        /*
        egui::Window::new("Code")
            .show(ctx, |ui| self.editor.ui(ui));
        */
        if let Some(code) = &self.code
            && self.label_menu
        {
            egui::Window::new("Labels").show(ctx, |ui| {});
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        ui.button("Save");
                        ui.button("Load");
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("Run", |ui| {
                        if ui.button("Compile").clicked() {
                            println!("code: {}", self.editor.code);
                            self.compile();
                        }

                        if ui.button("Compile & Run").clicked() {
                            self.compile_run().unwrap();
                        }

                        if self.code.is_some() {
                            ui.button("Call");
                            ui.button("Pause");
                            ui.button("Stop");
                            ui.button("Next");
                        }
                    });
                    if let Some(code) = &self.code {
                        ui.menu_button("Program", |ui| {
                            if ui.button("Labels").clicked() {
                                self.label_menu = true;
                            }
                        });
                    };

                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Color Scheme", |ui| {
                            egui::widgets::global_theme_preference_buttons(ui);
                        });
                    });
                    ui.menu_button("Help", |ui| {});
                    ui.add_space(30.0);
                }
            });
        });

        if let Some(code) = &mut self.code {
            egui::SidePanel::left("main_side_left").show(ctx, |ui| {
                ui.heading("⚙ Debug");
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                        ui.collapsing("⎈ Controls", |ui| {
                            ui.label(format!("PC: 0x{:04x}", code.interpreter.pc));
                            ui.horizontal(|ui| {
                                ui.button("▶ run");
                                ui.button("⏮ reset");
                                if ui.button("⏩ next").clicked() {
                                    code.interpreter.exec_next_op(&mut self.env).unwrap();
                                }
                            });
                            ui.separator();
                        });

                        
                        ui.collapsing("⛃ Value Stack", |ui| {
                            let stack = &code.interpreter.value_stack;
                            if stack.len() > 0 {
                                value_table(ui, stack, None, None);
                                ui.separator();
                            }
                        });


                        ui.collapsing("🌐 Globals", |ui| {
                            let slider_response = ui.add(
                                egui::Slider::new(&mut self.selected_global_slot_slider, 0..=63)
                                .logarithmic(true)
                                .text("Slot to scroll to"),
                            );

                            if slider_response.changed() {
                                self.selected_global_slot = Some(self.selected_global_slot_slider)
                            } else {
                                self.selected_global_slot = None;
                            }
                            ui.separator();

                            value_table(ui, &code.interpreter.globals, Some("Slot"), self.selected_global_slot);
                            ui.separator();
                            
                        });
                        ui.collapsing("Ｓ Frames", |ui| {
                            for (i, frame) in code.interpreter.return_stack.iter().enumerate() {
                                ui.collapsing(format!("{}: @0x{:04x}", i, frame.return_addr), |ui| {
                                    let slider_response = ui.add(
                                        egui::Slider::new(&mut self.selected_local_slot_slider, 0..=63)
                                        .logarithmic(true)
                                        .text("Slot to scroll to"),
                                    );

                                    if slider_response.changed() {
                                        self.selected_local_slot = Some(self.selected_local_slot_slider)
                                    } else {
                                        self.selected_local_slot = None;
                                    }
                                    ui.separator();

                                    value_table(ui, &frame.locals, Some("Slot"), self.selected_local_slot);
                                    ui.separator();
                                });
                            } 
                        });
                        ui.collapsing("🏷 Labels", |ui| {
                            if let Some(index) = select_label(ui, &code) {
                                let (name, position) = code.labels.get(index).unwrap();
                                self.jump_dest = Some(*position as usize);
                            }
                            ui.separator();
                            ui.horizontal(|ui| {
                                if let Some(dest) = self.jump_dest {
                                    ui.label(format!("Jump to: 0x{:04x}", dest));
                                    ui.button("jump");
                                    ui.button("call");
                                }
                            });
                        });
                        if code.results.len() > 0 {
                            ui.collapsing("✔ Results", |ui| {
                                ui.push_id(0, |ui| value_table(ui, &code.results, None, None));
                                ui.separator();
                            });
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("🖮 Editor");
            self.editor.ui(ui);

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        if let Some(code) = &self.code {
            egui::SidePanel::right("main_right_side").show(ctx, |ui| {
                ui.heading("⚡ Code");
                show_mem_op(ui, code);
            });

            egui::TopBottomPanel::bottom("main_bottom_side")
                .resizable(true)
                .show(ctx, |ui| {
                ui.heading("📝 Log");
                let text_response = ui.text_edit_multiline(&mut self.env.log.as_str());
                
            });
        };

    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
