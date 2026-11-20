//! Preferences ▸ Personal account: make a key, type or paste one made on another device, show
//! it to copy, or take it off this device (`collab.account`). Everything goes through the command;
//! the key shown is kept only while Preferences is open.

use egui::{Id, Ui};
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::theme::{Tokens, semibold};

/// Where the key shown stays while Preferences is open.
fn shown_id() -> Id {
    Id::unique("account_key_shown")
}

/// The Personal account part of Preferences.
pub(super) fn settings(app: &mut FigApp, ui: &mut Ui, t: &Tokens) {
    ui.label(egui::RichText::new(tr("Personal account")).font(semibold(12.0)).color(t.text));
    crate::widgets::label(
        ui,
        tr("One key on each of your devices shows the same online files on all of them. Keep it to yourself: it opens your files."),
        t.text_secondary,
    );
    let shown: Option<String> = ui.data(|d| d.get_temp(shown_id()));
    if app.session.account_key().is_none() {
        ui.horizontal(|ui| {
            if crate::widgets::button(ui, tr("Create key"), true, true).clicked()
                && let Some(r) = app.run("collab.account", json!({"create": true}))
                && let Some(key) = r["key"].as_str()
            {
                ui.data_mut(|d| d.insert_temp(shown_id(), key.to_string()));
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr("Key from another device"));
            let name = tr("Key from another device");
            if let Some(typed) = crate::widgets::text_field_named(ui, Id::unique("pref_account_key"), "", 220.0, name)
                && !typed.trim().is_empty()
            {
                app.run("collab.account", json!({"key": typed}));
            }
        });
        return;
    }
    crate::widgets::label(ui, tr("This device is in your account."), t.text_secondary);
    ui.horizontal(|ui| {
        match &shown {
            Some(key) => {
                ui.label(egui::RichText::new(key).monospace().color(t.text));
                if crate::widgets::button(ui, tr("Copy"), false, true).clicked() {
                    ui.ctx().copy_text(key.clone());
                }
                if crate::widgets::button(ui, tr("Hide key"), false, true).clicked() {
                    ui.data_mut(|d| d.remove::<String>(shown_id()));
                }
            }
            None => {
                if crate::widgets::button(ui, tr("Show key"), false, true).clicked()
                    && let Some(r) = app.run("collab.account", json!({"reveal": true}))
                    && let Some(key) = r["key"].as_str()
                {
                    ui.data_mut(|d| d.insert_temp(shown_id(), key.to_string()));
                }
            }
        }
        if crate::widgets::button(ui, tr("Leave account"), false, true).on_hover_text(tr("Take the key off this device")).clicked() {
            app.run("collab.account", json!({"clear": true}));
            ui.data_mut(|d| d.remove::<String>(shown_id()));
        }
    });
}

/// Preferences closed: the key shown is forgotten.
pub(super) fn forget_shown(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<String>(shown_id()));
}
