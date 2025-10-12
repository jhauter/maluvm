use egui_extras::{Column, TableBuilder};
use vm::asm::{self, Op};

pub struct Editor {
    code: String,
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
                    .code_editor()
                    .desired_rows(5)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        });
    }
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
