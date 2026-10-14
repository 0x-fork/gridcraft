//! Format Painter through the real ribbon button and a click on the grid.

use egui::accesskit::Toggled;
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{NodeT, Queryable};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_engine::model::Style;
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [["source"]]})).unwrap();
    session.execute("home.bold", json!({"range": "A1", "on": true})).unwrap();
    session.execute("home.fillColor", json!({"range": "A1", "color": "#FFFF00"})).unwrap();
    session.execute("selection.set", json!({"cell": "A1"})).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        SheetApp::new(session, Default::default()),
    );
    h.run_steps(4);
    h
}

fn style(h: &Harness, a1: &str) -> Style {
    let doc = h.state().session.active().unwrap();
    let sheet = doc.wb.active().unwrap();
    doc.wb.styles.get(sheet.style_id(CellRef::parse(a1).unwrap())).clone()
}

fn point(h: &Harness, a1: &str) -> Pos2 {
    let app = h.state();
    let sheet = app.session.active().unwrap().wb.active().unwrap();
    Geo::new(sheet, app.grid.rect.unwrap(), app.view().scroll).cell_rect(sheet, CellRef::parse(a1).unwrap()).center()
}

fn pressed(h: &Harness) -> bool {
    h.get_by_label("Format Painter").accesskit_node().toggled() == Some(Toggled::True)
}

fn click_cell(h: &mut Harness, a1: &str) {
    let at = point(h, a1);
    for event in [
        Event::PointerMoved(at),
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
    ] {
        h.input_mut().events.push(event);
        h.run_steps(1);
    }
    h.run_steps(2);
}

#[test]
fn ribbon_button_picks_up_the_format_and_a_click_on_a_cell_pastes_it() {
    let mut h = harness();
    let source = style(&h, "A1");
    assert!(source.font.bold);
    assert_ne!(style(&h, "C3"), source);

    h.get_by_label("Format Painter").click();
    h.run_steps(2);
    assert!(h.state().session.format_painter.is_some(), "the button must arm the painter");
    assert!(pressed(&h), "the armed button must show as pressed");

    click_cell(&mut h, "C3");
    assert_eq!(style(&h, "C3"), source, "clicking C3 must paste A1's format");
    assert!(h.state().session.format_painter.is_none(), "a single-click painter disarms after one use");
    assert!(!pressed(&h));
}
