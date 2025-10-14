use vm::{asm, interpreter::Interpreter};

use crate::code::{self, result_table, Editor};

pub struct TemplateApp {
    // Example stuff:
    label: String,
    value: f32,
    editor: Editor,
    interpreter: Option<Interpreter>,
    results: Vec<u32> 
}
impl TemplateApp {
    fn compile(&mut self) {
        //TODO: Error Handling 
        let text = &self.editor.code;
        let bytecode = asm::Parser::parse(&text).unwrap(); 

        match self.interpreter {
            Some(ref mut interpreter) => {
                interpreter.reset_all(&bytecode).unwrap();
            }
            None => {
                self.interpreter = Some(Interpreter::from_bytecode(&bytecode).unwrap());
            }
        } 
    }

    
    fn compile_run(&mut self) {
        self.compile();
        let i = self.interpreter.as_mut().unwrap(); 
        i.run().unwrap().clone_into(&mut self.results);
    }
}
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            editor: Default::default(),
            interpreter: None,
            results: Vec::new(),
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
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        /*
        egui::Window::new("Code")
            .show(ctx, |ui| self.editor.ui(ui));
        */
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
                            self.compile_run()
                        }
                        ui.button("Pause");
                        ui.button("Stop");
                        ui.button("Next");
                    });
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Editor");
            self.editor.ui(ui);
            if self.results.len() > 0 {
                result_table(ui, &self.results); 
            }
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
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
