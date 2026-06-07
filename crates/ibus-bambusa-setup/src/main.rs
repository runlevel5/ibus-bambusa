//! Preferences GUI for ibus-bambusa.
//!
//! A small libadwaita window bound directly to the GSettings (dconf) schema the
//! engine reads. Switches use `Settings::bind`, so toggling a row writes the
//! key immediately; the engine picks it up on the next focus.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use bambusa_config::{SCHEMA_ID, keys};
use bambusa_core::charset_names;
use gettextrs::{
    LocaleCategory, bind_textdomain_codeset, bindtextdomain, gettext, setlocale, textdomain,
};
use gtk::gio::prelude::*;
use gtk::gio::{ApplicationFlags, Settings};
use gtk::glib::ExitCode;
use gtk::{StringList, glib};
use libadwaita as adw;

/// Editable macro rows in the editor: `(container, shortcut, expansion)`.
type MacroRows = Rc<RefCell<Vec<(gtk::Box, gtk::Entry, gtk::Entry)>>>;
/// Shared callback that appends a `(shortcut, expansion)` row to the editor.
type AddRow = Rc<dyn Fn(&str, &str)>;

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
    page.add(&build_macros_group(&settings));

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

/// The "Macros" group: the two toggles plus an "edit" button that opens the
/// macro editor window.
fn build_macros_group(settings: &Settings) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(gettext("Macros"))
        .build();

    add_switch(
        &group,
        settings,
        "Enable macros",
        "Expand a shortcut to its full text on Space or Tab.",
        keys::MACROS_ENABLED,
    );
    add_switch(
        &group,
        settings,
        "Auto-capitalize macros",
        "Match the expansion's case to how you type the shortcut.",
        keys::AUTO_CAPITALIZE_MACROS,
    );

    let edit = gtk::Button::builder()
        .icon_name("document-edit-symbolic")
        .tooltip_text(gettext("Edit macros"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    edit.connect_clicked({
        let settings = settings.clone();
        move |button| {
            let parent = button.root().and_downcast::<gtk::Window>();
            open_macro_editor(&settings, parent.as_ref());
        }
    });
    group.set_header_suffix(Some(&edit));

    group
}

/// A modal editor window: a two-column (Shortcut / Expansion) list of editable
/// rows, each with a remove button, over an Add / Import / Export button row.
/// Changes are saved back to GSettings when the window closes.
fn open_macro_editor(settings: &Settings, parent: Option<&gtk::Window>) {
    let window = adw::Window::builder()
        .title(gettext("Edit macros"))
        .modal(true)
        .default_width(520)
        .default_height(440)
        .build();
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }

    let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    // Tracked rows: (container, shortcut entry, expansion entry).
    let rows: MacroRows = Rc::new(RefCell::new(Vec::new()));

    let add_row: AddRow = {
        let list = list.clone();
        let rows = rows.clone();
        Rc::new(move |key: &str, value: &str| {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let key_entry = gtk::Entry::builder()
                .text(key)
                .placeholder_text(gettext("Shortcut"))
                .hexpand(true)
                .build();
            let value_entry = gtk::Entry::builder()
                .text(value)
                .placeholder_text(gettext("Expansion"))
                .hexpand(true)
                .build();
            let del = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text(gettext("Remove"))
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();
            row.append(&key_entry);
            row.append(&value_entry);
            row.append(&del);
            del.connect_clicked({
                let list = list.clone();
                let rows = rows.clone();
                let row = row.clone();
                move |_| {
                    list.remove(&row);
                    rows.borrow_mut().retain(|(r, _, _)| *r != row);
                }
            });
            list.append(&row);
            rows.borrow_mut().push((row, key_entry, value_entry));
        })
    };

    for (key, value) in parse_macros(settings) {
        add_row(&key, &value);
    }

    // Add / Import / Export.
    let add_btn = gtk::Button::with_label(&gettext("Add"));
    let import_btn = gtk::Button::with_label(&gettext("Import macros"));
    let export_btn = gtk::Button::with_label(&gettext("Export macros"));
    add_btn.connect_clicked({
        let add_row = add_row.clone();
        move |_| add_row("", "")
    });
    import_btn.connect_clicked({
        let add_row = add_row.clone();
        let window = window.clone();
        move |_| {
            let dialog = gtk::FileDialog::builder()
                .title(gettext("Import macros"))
                .build();
            let add_row = add_row.clone();
            dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res
                    && let Some(path) = file.path()
                    && let Ok(content) = std::fs::read_to_string(path)
                {
                    for line in content.lines() {
                        let s = line.trim();
                        if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                            continue;
                        }
                        if let Some((k, v)) = s.split_once(':') {
                            add_row(k.trim(), v.trim());
                        }
                    }
                }
            });
        }
    });
    export_btn.connect_clicked({
        let rows = rows.clone();
        let window = window.clone();
        move |_| {
            let dialog = gtk::FileDialog::builder()
                .title(gettext("Export macros"))
                .initial_name("macro.text")
                .build();
            let rows = rows.clone();
            dialog.save(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res
                    && let Some(path) = file.path()
                {
                    let mut out = String::new();
                    for (_, key, value) in rows.borrow().iter() {
                        let key = key.text();
                        let key = key.trim();
                        if !key.is_empty() {
                            out.push_str(&format!("{key}:{}\n", value.text().trim()));
                        }
                    }
                    let _ = std::fs::write(path, out);
                }
            });
        }
    });

    window.connect_close_request({
        let settings = settings.clone();
        let rows = rows.clone();
        move |_| {
            let pairs: Vec<(String, String)> = rows
                .borrow()
                .iter()
                .filter_map(|(_, key, value)| {
                    let key = key.text().trim().to_string();
                    let value = value.text().trim().to_string();
                    (!key.is_empty() && !value.is_empty()).then_some((key, value))
                })
                .collect();
            write_macros(&settings, &pairs);
            glib::Propagation::Proceed
        }
    });

    // Column headers.
    let headers = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let h_key = gtk::Label::builder()
        .label(gettext("Shortcut"))
        .halign(gtk::Align::Start)
        .hexpand(true)
        .css_classes(["dim-label"])
        .build();
    let h_value = gtk::Label::builder()
        .label(gettext("Expansion"))
        .halign(gtk::Align::Start)
        .hexpand(true)
        .css_classes(["dim-label"])
        .build();
    headers.append(&h_key);
    headers.append(&h_value);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    buttons.append(&add_btn);
    buttons.append(&import_btn);
    buttons.append(&export_btn);

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&headers);
    content.append(&scroll);
    content.append(&buttons);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&content));
    window.set_content(Some(&toolbar));
    window.present();
}

/// Read the macros from GSettings as `(shortcut, expansion)` pairs.
fn parse_macros(settings: &Settings) -> Vec<(String, String)> {
    settings
        .strv(keys::MACROS)
        .iter()
        .filter_map(|entry| {
            let entry = entry.to_string();
            entry
                .split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect()
}

/// Write the macros back to GSettings as `"shortcut:expansion"` strings.
fn write_macros(settings: &Settings, pairs: &[(String, String)]) {
    let entries: Vec<String> = pairs.iter().map(|(k, v)| format!("{k}:{v}")).collect();
    let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
    let _ = settings.set_strv(keys::MACROS, refs.as_slice());
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
