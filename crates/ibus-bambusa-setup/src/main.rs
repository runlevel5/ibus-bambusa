//! Preferences GUI for ibus-bambusa.
//!
//! A small libadwaita window bound directly to the GSettings (dconf) schema the
//! engine reads. Switches use `Settings::bind`, so toggling a row writes the
//! key immediately; the engine picks it up on the next focus.

use adw::prelude::*;
use bambusa_config::{SCHEMA_ID, keys};
use bambusa_core::charset_names;
use gettextrs::{
    LocaleCategory, bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain,
};
use gtk::StringList;
use gtk::gio::prelude::*;
use gtk::gio::{ApplicationFlags, Settings};
use gtk::glib::ExitCode;
use libadwaita as adw;

const APP_ID: &str = "org.freedesktop.IBus.bambusa.setup";
const GETTEXT_DOMAIN: &str = "ibus-bambusa";
const LOCALEDIR: &str = "/usr/share/locale";

fn init_i18n() {
    setlocale(LocaleCategory::LcAll, "");
    let _ = bindtextdomain(GETTEXT_DOMAIN, LOCALEDIR);
    let _ = bind_textdomain_codeset(GETTEXT_DOMAIN, "UTF-8");
    let _ = textdomain(GETTEXT_DOMAIN);
}

fn main() {
    init_i18n();
    // IBus and GNOME launch the setup with an extra argv[1] (the basename, per
    // ibus-setup's convention); HANDLES_COMMAND_LINE lets us accept and ignore
    // any arguments instead of bailing out as the default flags would.
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.connect_command_line(|app, _cmdline| {
        app.activate();
        ExitCode::SUCCESS
    });
    app.connect_activate(|app| {
        // Single-instance: launching again (clicking Preferences repeatedly)
        // re-activates the running process, so just raise the existing window
        // instead of opening another.
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            build_ui(app);
        }
    });
    app.run();
}

fn build_ui(app: &adw::Application) {
    let settings = Settings::new(SCHEMA_ID);
    let page = adw::PreferencesPage::new();

    // Output charset.
    let output = group("Output");
    let charsets = charset_names();
    let charset_row = adw::ComboRow::builder()
        .title(gettext("Character set"))
        .model(&StringList::new(&charsets))
        .build();
    let current: String = settings.string(keys::OUTPUT_CHARSET).into();
    if let Some(idx) = charsets.iter().position(|c| *c == current.as_str()) {
        charset_row.set_selected(idx as u32);
    }
    {
        let settings = settings.clone();
        charset_row.connect_selected_notify(move |row| {
            if let Some(name) = charset_names().get(row.selected() as usize) {
                let _ = settings.set_string(keys::OUTPUT_CHARSET, name);
            }
        });
    }
    output.add(&charset_row);
    page.add(&output);

    // Input mode. Enum values: preedit=1, surrounding-text=2 (others not yet
    // exposed in the UI).
    const MODES: [i32; 2] = [1, 2];
    let input = group("Input");
    let preedit_label = gettext("Preedit");
    let surrounding_label = gettext("Surrounding text");
    let mode_row = adw::ComboRow::builder()
        .title(gettext("Input mode"))
        .model(&StringList::new(&[
            preedit_label.as_str(),
            surrounding_label.as_str(),
        ]))
        .build();
    let cur = settings.enum_(keys::INPUT_MODE);
    mode_row.set_selected(MODES.iter().position(|&m| m == cur).unwrap_or(0) as u32);
    {
        let settings = settings.clone();
        mode_row.connect_selected_notify(move |row| {
            if let Some(&m) = MODES.get(row.selected() as usize) {
                let _ = settings.set_enum(keys::INPUT_MODE, m);
            }
        });
    }
    input.add(&mode_row);
    page.add(&input);

    // Tone marking.
    let tones = group("Tone marking");
    add_switch(
        &tones,
        &settings,
        "Free tone marking",
        "Accept tone marks anywhere in the word, not only after the vowel.",
        keys::FREE_TONE_MARKING,
    );
    add_switch(
        &tones,
        &settings,
        "Modern tone placement",
        "Place the tone on the second vowel in cases like “oa”, “uy” (hoà, not hòa).",
        keys::MODERN_TONE_STYLE,
    );
    page.add(&tones);

    // Spell checking.
    let spell = group("Spell checking");
    add_switch(
        &spell,
        &settings,
        "Check spelling",
        "Fall back to the raw keystrokes when a word is not valid Vietnamese.",
        keys::SPELL_CHECK,
    );
    add_switch(
        &spell,
        &settings,
        "Use spelling rules",
        "Accept syllables that follow Vietnamese spelling rules.",
        keys::SPELL_CHECK_RULES,
    );
    add_switch(
        &spell,
        &settings,
        "Use dictionary",
        "Also accept whole words found in the bundled dictionary.",
        keys::SPELL_CHECK_DICTS,
    );
    page.add(&spell);

    // Behaviour.
    let behaviour = group("Behaviour");
    add_switch(
        &behaviour,
        &settings,
        "Restore non-Vietnamese words",
        "Undo composition automatically for words that are not Vietnamese.",
        keys::AUTO_RESTORE_NON_VN,
    );
    add_switch(
        &behaviour,
        &settings,
        "Hide preedit underline",
        "",
        keys::HIDE_UNDERLINE,
    );
    page.add(&behaviour);

    // Macros.
    let macros = group("Macros");
    add_switch(
        &macros,
        &settings,
        "Enable macros",
        "",
        keys::MACROS_ENABLED,
    );
    add_switch(
        &macros,
        &settings,
        "Auto-capitalize macros",
        "",
        keys::AUTO_CAPITALIZE_MACROS,
    );
    page.add(&macros);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&page));

    adw::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Bambusa Preferences"))
        .default_width(460)
        .default_height(640)
        .content(&toolbar)
        .build()
        .present();
}

fn group(title: &str) -> adw::PreferencesGroup {
    adw::PreferencesGroup::builder()
        .title(gettext(title))
        .build()
}

/// A switch row bound two-way to a boolean GSettings key.
fn add_switch(
    group: &adw::PreferencesGroup,
    settings: &Settings,
    title: &str,
    subtitle: &str,
    key: &str,
) {
    let row = adw::SwitchRow::builder().title(gettext(title)).build();
    if !subtitle.is_empty() {
        row.set_subtitle(&gettext(subtitle));
    }
    settings.bind(key, &row, "active").build();
    group.add(&row);
}
