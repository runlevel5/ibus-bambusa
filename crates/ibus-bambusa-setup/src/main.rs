//! Preferences GUI for ibus-bambusa.
//!
//! A small libadwaita window over the shared `Config`. Each row writes the
//! change straight back to the config file the engine reads, so settings take
//! effect on the next focus/restart.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use bambusa_config::{Config, IBFlags};
use bambusa_core::{EngineFlags, charset_names};
use gtk::StringList;
use libadwaita as adw;

const APP_ID: &str = "org.freedesktop.IBus.bambusa.setup";

type Shared = Rc<RefCell<Config>>;

fn main() {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let config: Shared = Rc::new(RefCell::new(Config::load()));
    let page = adw::PreferencesPage::new();

    // Output charset.
    let output = adw::PreferencesGroup::builder().title("Output").build();
    let charsets = charset_names();
    let charset_row = adw::ComboRow::builder()
        .title("Character set")
        .model(&StringList::new(&charsets))
        .build();
    if let Some(idx) = charsets
        .iter()
        .position(|c| *c == config.borrow().output_charset)
    {
        charset_row.set_selected(idx as u32);
    }
    {
        let config = config.clone();
        charset_row.connect_selected_notify(move |row| {
            if let Some(name) = charset_names().get(row.selected() as usize) {
                config.borrow_mut().output_charset = name.to_string();
                let _ = config.borrow().save();
            }
        });
    }
    output.add(&charset_row);
    page.add(&output);

    // Tone marking.
    let tones = adw::PreferencesGroup::builder()
        .title("Tone marking")
        .build();
    engine_switch(
        &tones,
        &config,
        "Free tone marking",
        "Accept tone marks anywhere in the word, not only after the vowel.",
        EngineFlags::FREE_TONE_MARKING,
    );
    engine_switch(
        &tones,
        &config,
        "Modern tone placement",
        "Place the tone on the second vowel in cases like “oa”, “uy” (hoà, not hòa).",
        EngineFlags::STD_TONE_STYLE,
    );
    page.add(&tones);

    // Spell checking.
    let spell = adw::PreferencesGroup::builder()
        .title("Spell checking")
        .build();
    ib_switch(
        &spell,
        &config,
        "Check spelling",
        "Fall back to the raw keystrokes when a word is not valid Vietnamese.",
        IBFlags::SPELL_CHECK_ENABLED,
    );
    ib_switch(
        &spell,
        &config,
        "Use spelling rules",
        "",
        IBFlags::SPELL_CHECK_WITH_RULES,
    );
    ib_switch(
        &spell,
        &config,
        "Use dictionary",
        "",
        IBFlags::SPELL_CHECK_WITH_DICTS,
    );
    page.add(&spell);

    // Behaviour.
    let behaviour = adw::PreferencesGroup::builder().title("Behaviour").build();
    ib_switch(
        &behaviour,
        &config,
        "Restore non-Vietnamese words",
        "Undo composition automatically for words that are not Vietnamese.",
        IBFlags::AUTO_NON_VN_RESTORE,
    );
    ib_switch(
        &behaviour,
        &config,
        "Hide preedit underline",
        "",
        IBFlags::NO_UNDERLINE,
    );
    page.add(&behaviour);

    // Macros.
    let macros = adw::PreferencesGroup::builder().title("Macros").build();
    ib_switch(
        &macros,
        &config,
        "Enable macros",
        "",
        IBFlags::MACRO_ENABLED,
    );
    ib_switch(
        &macros,
        &config,
        "Auto-capitalize macros",
        "",
        IBFlags::AUTO_CAPITALIZE_MACRO,
    );
    page.add(&macros);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));

    adw::ApplicationWindow::builder()
        .application(app)
        .title("Bambusa Preferences")
        .default_width(460)
        .default_height(640)
        .content(&toolbar)
        .build()
        .present();
}

/// A switch row bound to an `IBFlags` bit.
fn ib_switch(
    group: &adw::PreferencesGroup,
    config: &Shared,
    title: &str,
    subtitle: &str,
    flag: IBFlags,
) {
    let row = adw::SwitchRow::builder()
        .title(title)
        .active(config.borrow().ib_flags.contains(flag))
        .build();
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }
    let config = config.clone();
    row.connect_active_notify(move |row| {
        config.borrow_mut().ib_flags.set(flag, row.is_active());
        let _ = config.borrow().save();
    });
    group.add(&row);
}

/// A switch row bound to an `EngineFlags` bit.
fn engine_switch(
    group: &adw::PreferencesGroup,
    config: &Shared,
    title: &str,
    subtitle: &str,
    flag: EngineFlags,
) {
    let row = adw::SwitchRow::builder()
        .title(title)
        .active(config.borrow().engine_flags.contains(flag))
        .build();
    if !subtitle.is_empty() {
        row.set_subtitle(subtitle);
    }
    let config = config.clone();
    row.connect_active_notify(move |row| {
        config.borrow_mut().engine_flags.set(flag, row.is_active());
        let _ = config.borrow().save();
    });
    group.add(&row);
}
