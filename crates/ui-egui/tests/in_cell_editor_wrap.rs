//! The in-cell editor of a Wrap Text cell keeps the column's width and wraps, growing down, as
//! Excel's does; it used to widen to one long line.
use egui::{Context, Id, RawInput, Rect, pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

const TEXT: &str = "Since the 1960s, NASA has left hardware across the solar system, on the Moon, on Mars, and in deep space.";

/// Runs a few frames of the app in a 1200 × 800 window.
fn frames(ctx: &Context, app: &mut SheetApp) {
    for _ in 0..4 {
        let mut output =
            ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), ..Default::default() }, |ui| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            });
        output.textures_delta.clear();
    }
}

/// The editor's text field rect while A1 (narrow, wrap on or off) is edited with [`TEXT`].
fn editor_rect(wrap: bool) -> Rect {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("home.columnWidth", json!({"cols": "A:A", "chars": 12})).unwrap();
    session.execute("home.wrapText", json!({"range": "A1", "on": wrap})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    frames(&ctx, &mut app);
    app.begin_edit(Some(TEXT.into()), false);
    frames(&ctx, &mut app);
    ctx.read_response(Id::new("gridcraft.cell_editor")).expect("the in-cell editor is shown").rect
}

#[test]
fn a_wrap_text_cell_edits_wrapped_at_the_column_width() {
    let wrapped = editor_rect(true);
    let one_line = editor_rect(false);
    assert!(one_line.width() > 400.0, "without Wrap Text the editor widens to the text: {one_line:?}");
    assert!(wrapped.width() < 120.0, "with Wrap Text the editor keeps the column's width: {wrapped:?}");
    assert!(wrapped.height() > 4.0 * one_line.height(), "and grows down to show every line: {wrapped:?} vs {one_line:?}");
}

/// A short Wrap Text edit stays one line tall: the field doesn't reserve rows over the cells below.
#[test]
fn a_short_wrap_text_edit_stays_one_line_tall() {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("home.wrapText", json!({"range": "A1", "on": true})).unwrap();
    let mut app = SheetApp::new(session, Default::default());
    frames(&ctx, &mut app);
    app.begin_edit(Some("short".into()), false);
    frames(&ctx, &mut app);
    let field = ctx.read_response(Id::new("gridcraft.cell_editor")).expect("the in-cell editor is shown").rect;
    assert!(field.height() < 30.0, "a one-line edit doesn't reach over the rows below: {field:?}");
}
