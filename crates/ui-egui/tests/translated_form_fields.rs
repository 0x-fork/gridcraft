//! Form dialogs draw their field labels and choices in the interface language, as their titles
//! and OK/Cancel buttons already were (Page Setup showed "Left margin (in):" in every language).

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::SheetApp;
use serde_json::json;

fn harness() -> egui_kittest::Harness<'static, SheetApp> {
    let mut session = Session::new();
    session.new_workbook();
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
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

#[test]
fn form_fields_are_translated() {
    for (dialog, translated, english) in [
        ("pageSetup", ["Margem esquerda (pol.):", "Rodapé (pol.):"], ["Left margin (in):", "Footer (in):"]),
        ("insertPaste", ["Deslocar células para a direita", "Deslocar células para baixo"], ["Shift cells right", "Shift cells down"]),
    ] {
        let mut h = harness();
        h.state_mut().run("app.language.set", json!({"language": "pt"})).unwrap();
        h.state_mut().open_dialog(dialog, json!({}));
        h.run_steps(3);
        for label in translated {
            assert!(h.query_by_label_contains(label).is_some(), "{dialog}: no {label}");
        }
        for label in english {
            assert!(h.query_by_label_contains(label).is_none(), "{dialog}: {label} is untranslated");
        }
    }
}
