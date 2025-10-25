
use std::io::Cursor;

use egui::{ahash::HashMap, text::LayoutJob, Color32, ScrollArea, TextFormat, TextStyle};
use egui_extras::{Column, TableBuilder};
use vm::{asm::{self, Op, RawOp}, parse::{try_parse_ops_from_bytecode, MaybeRawOp}};

use crate::app::CompiledCode;

pub struct Editor {
    pub code: String,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            code: include_str!("../assets/asm/code_example.malu").into(),
        }
    }
}
impl Editor {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        //TODO: Syntax Highlighting für asm
        let mut theme =
            egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());

        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                ui.ctx(),
                ui.style(),
                &theme,
                buf.as_str(),
                "rs",
            );
            layout_job.wrap.max_width = wrap_width;
            
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut self.code)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Enter your code here")
                    .code_editor()
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .clip_text(true)
                    .layouter(&mut layouter),
                     
            );
        });
    }
}

pub fn value_table(ui: &mut egui::Ui, results: &[u32], index_name: Option<&str>, row_index: Option<usize>) {
    let text_height = egui::TextStyle::Body
        .resolve(ui.style())
        .size
        .max(ui.spacing().interact_size.y);
    

    let mut table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .min_scrolled_height(0.0)
        .max_scroll_height(100.0)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto());

    if index_name.is_some() {
        table = table.column(Column::auto())
    }

    if let Some(row) = row_index {
        table = table.scroll_to_row(row, None)
    }

    table.header(20.0, |mut header| {
        if let Some(i) = index_name {
            header.col(|ui| {
                ui.strong(i);
            });
        }
        header.col(|ui| {
            ui.strong("Signed");
        });
        header.col(|ui| {
            ui.strong("Unsigned");
        });
        header.col(|ui| {
            ui.strong("Hex");
        });
    })
    .body(|body| {
        body.rows(text_height, results.len(), |mut row| {
            let row_index = row.index();
            let result = results[row_index];

            if index_name.is_some() {
                row.col(|ui| {
                    ui.label(format!("{}", row_index));
                });
    
            }
            row.col(|ui| {
                ui.label(format!("{}", result as i32));
            });
            row.col(|ui| {
                ui.label(format!("{}", result));
            });
            row.col(|ui| {
                ui.label(format!("0x{:04x}", result));
            });
        });
    });
}

pub fn select_label<'a>(ui: &mut egui::Ui, code: &'a CompiledCode) -> Option<usize> {
    let mut selected = None;
    let text_height = egui::TextStyle::Body
        .resolve(ui.style())
        .size
        .max(ui.spacing().interact_size.y);

    let available_height = ui.available_height();
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .min_scrolled_height(0.0)
        .max_scroll_height(available_height)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto());
        
    table
        .sense(egui::Sense::click()) 
        .header(20.0, |mut header| {
        header.col(|ui| {
            ui.strong("Index");
        });
        header.col(|ui| {
            ui.strong("Label");
        });
        header.col(|ui| {
            ui.strong("Position");
        });
    })
    .body(|body| {
        let labels = &code.labels;
        let mut iter = labels.iter();

        body.rows(text_height, labels.len(), |mut row| {
            let row_index = row.index();
            let (name, position) = iter.next().unwrap();
            row.col(|ui| {
                ui.label(row_index.to_string());
            });
            row.col(|ui| {
                ui.label(format!("{}", name));
            });
            row.col(|ui| {
                ui.label(format!("0x{:04x}", position));
            });
            if row.response().clicked() {
                println!("clicked {}", name);
                selected = Some(row_index);
            }
        });
    });

    selected
}

pub fn show_mem_op(ui: &mut egui::Ui, code: &CompiledCode) {
    ScrollArea::vertical().id_salt("grid_scroll").show(ui, |ui| {
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);
        let pc = code.interpreter.pc;
        let available_height = ui.available_height();

        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto());
        table.header(10.0, |mut header| {
            header.col(|ui| {
                ui.strong("Offset");
            });
            header.col(|ui| {
                ui.strong("Opcode");
            });
            header.col(|ui| {
                ui.strong("Argument");
            });
        })
        .body(|body| {
            body.rows(text_height, code.ops.len(), |mut row| {
                let (op, offset) = &code.ops[row.index()];
                row.set_selected(pc as usize == *offset as usize);
                row.col(|ui| {
                    ui.label(format!("0x{:04x}", offset));
                });
                row.col(|ui| {
                    match op {
                        MaybeRawOp::Op(raw_op) => {
                            ui.label(format!("{}", raw_op.name()));
                        },
                        MaybeRawOp::Unknown(_) => {
                            ui.label("???");
                        },
                    }
                });
                match op {
                    MaybeRawOp::Op(RawOp {arg: Some(arg), ..}) => {
                        row.col(|ui| {
                            ui.label(format!("{}", arg));
                        });
                    }
                    MaybeRawOp::Unknown(v) => {
                        row.col(|ui| {
                            ui.label(format!("0x{:04x}", v));
                        });
                    }
                    _ => {

                    }
                }
            }); 
        });  

    //     egui::Grid::new("code grid")
    //         .striped(true)
    //         .show(ui, |ui| {
    //         for (op, offset) in &code.ops {
    //             ui.label(format!("0x{:04x}", offset));
    //             match op {
    //                 MaybeRawOp::Op(o) => {
    //                     ui.label(format!("{}", o.name()));
    //                     if let Some(a) = &o.arg {
    //                         ui.label(format!("{}", a));
    //                     }
    //                 },
    //                 MaybeRawOp::Unknown(u) => {
    //                     ui.label("???");
    //                     ui.label(format!("0x{:04x}", u));
    //                 }
    //             }
    //             ui.end_row();
    //         }
    //     });
    });
}
/*
pub fn ui_op_table<'src>(ops: &[Op<'src>], ui: &mut egui::Ui) {
    let text_height = egui::TextStyle::Body
        .resolve(ui.style())
        .size
        .max(ui.spacing().interact_size.y);

    let mut current_offset = 0;
    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .min_scrolled_height(0.0);

    table.header(20.0, |mut header| {
        header.col(|ui| {
            ui.strong("Index");
        });

        header.col(|ui| {
            ui.strong("Offset");
        });

        header.col(|ui| {
            ui.strong("Instruction");
        });
    })
    .body(|body| {
        body.rows(text_height, ops.len(), |mut row| {
            let row_index = row.index();
            let op = &ops[row_index];
            row.col(|ui| {
                ui.label(row_index.to_string());
            });
            row.col(|ui| {
                ui.label(format!("{:5x}", current_offset));
            });
            row.col(|ui| {
                ui.label(op.to_string());
            });

            current_offset += op.repr();
        });
    });
}
*/
