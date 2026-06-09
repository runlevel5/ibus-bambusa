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
    install_keycap_css();
    let settings = Settings::new(SCHEMA_ID);
    let page = adw::PreferencesPage::new();

    // Output charset. The internal names are the encoder keys / stored values;
    // the list shows translatable display labels for them.
    let output = group("Output");
    let charsets = charset_names();
    let labels: Vec<String> = charsets.iter().map(|c| charset_label(c)).collect();
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    let charset_row = adw::ComboRow::builder()
        .title(gettext("Character set"))
        .model(&StringList::new(&label_refs))
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

    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&page));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(gettext("Bambusa Preferences"))
        .default_width(460)
        .default_height(640)
        .content(&toolbar)
        .build();

    // Primary menu (hamburger) next to the window controls.
    let menu = gtk::gio::Menu::new();
    menu.append(Some(&gettext("Help")), Some("win.help"));
    menu.append(Some(&gettext("About")), Some("win.about"));
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(gettext("Main menu"))
        .menu_model(&menu)
        .build();
    header.pack_end(&menu_button);

    let about = gtk::gio::SimpleAction::new("about", None);
    about.connect_activate({
        let window = window.clone();
        move |_, _| show_about(&window)
    });
    window.add_action(&about);

    let help = gtk::gio::SimpleAction::new("help", None);
    help.connect_activate({
        let window = window.clone();
        move |_, _| open_help(&window)
    });
    window.add_action(&help);

    window.present();
}

/// Show the standard libadwaita About window.
fn show_about(parent: &impl IsA<gtk::Window>) {
    let about = adw::AboutWindow::builder()
        .transient_for(parent)
        .modal(true)
        .application_name("Bambusa")
        .application_icon("input-keyboard")
        .version(env!("CARGO_PKG_VERSION"))
        // The main-page subtitle: use the description rather than a name.
        .developer_name(gettext("A Vietnamese input method for GNOME."))
        .copyright("© 2025–2026 Trung Lê")
        .license_type(gtk::License::Gpl30)
        .comments(gettext("A Vietnamese input method for GNOME."))
        .website("https://github.com/runlevel5/ibus-bambusa")
        .issue_url("https://github.com/runlevel5/ibus-bambusa/issues")
        .build();

    // Credits page (shown above Legal).
    about.add_credit_section(Some(&gettext("Coded by")), &["Lê Đức Trung"]);
    about.add_credit_section(
        Some(&gettext("Based on ibus-bamboo by")),
        &["Lương Thanh Lâm"],
    );
    about.add_credit_section(
        Some(&gettext("Dictionary by")),
        &[
            "Hồ Ngọc Đức",
            "Ngô Quốc Hưng",
            "Free Vietnamese Dictionary Project",
            "Vietnamese Wiktionary",
        ],
    );

    about.present();
}

/// Per-method typing reference: `(method, [(effect, space-separated keys)])`.
/// Effects are translatable (the Vietnamese tone names are the `vi` strings);
/// letters and method names are shown verbatim.
const HELP: &[(&str, &[(&str, &str)])] = &[
    (
        "Telex",
        &[
            ("Acute tone (e.g. á)", "Vowel + S"),
            ("Grave tone (e.g. à)", "Vowel + F"),
            ("Hook above tone (e.g. ả)", "Vowel + R"),
            ("Tilde tone (e.g. ã)", "Vowel + X"),
            ("Dot below tone (e.g. ạ)", "Vowel + J"),
            ("Remove tone", "Vowel + Z"),
            ("â", "A + A"),
            ("ê", "E + E"),
            ("ô", "O + O"),
            ("ă", "A + W"),
            ("ơ", "O + W"),
            ("ư", "U + W"),
            ("đ", "D + D"),
        ],
    ),
    (
        "VNI",
        &[
            ("Acute tone (e.g. á)", "Vowel + 1"),
            ("Grave tone (e.g. à)", "Vowel + 2"),
            ("Hook above tone (e.g. ả)", "Vowel + 3"),
            ("Tilde tone (e.g. ã)", "Vowel + 4"),
            ("Dot below tone (e.g. ạ)", "Vowel + 5"),
            ("Circumflex (e.g. â ê ô)", "Vowel + 6"),
            ("Horn (e.g. ơ ư)", "Vowel + 7"),
            ("Breve (e.g. ă)", "Vowel + 8"),
            ("đ", "D + 9"),
            ("Remove tone", "Vowel + 0"),
        ],
    ),
    (
        "VIQR",
        &[
            ("Acute tone (e.g. á)", "Vowel + '"),
            ("Grave tone (e.g. à)", "Vowel + `"),
            ("Hook above tone (e.g. ả)", "Vowel + ?"),
            ("Tilde tone (e.g. ã)", "Vowel + ~"),
            ("Dot below tone (e.g. ạ)", "Vowel + ."),
            ("Circumflex (e.g. â ê ô)", "Vowel + ^"),
            ("Horn (e.g. ơ ư)", "Vowel + +"),
            ("Breve (e.g. ă)", "Vowel + ("),
            ("đ", "D + D"),
            ("Remove tone", "Vowel + -"),
        ],
    ),
];

/// Show a Help window: one section per input method, each row pairing the effect
/// with the keystrokes (rendered as keycaps).
fn open_help(parent: &impl IsA<gtk::Window>) {
    let window = adw::Window::builder()
        .title(gettext("How to type"))
        .modal(true)
        .transient_for(parent)
        .default_width(460)
        .default_height(620)
        .build();

    // The scrollable list of sections.
    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(6)
        .margin_bottom(18)
        .margin_start(12)
        .margin_end(12)
        .build();
    let clamp = adw::Clamp::builder().child(&list).build();
    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build();

    // Tag pills that jump to each section.
    let pills = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .halign(gtk::Align::Center)
        .homogeneous(false)
        .row_spacing(6)
        .column_spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(12)
        .margin_end(12)
        .build();

    let mut groups = Vec::new();
    let mut buttons = Vec::new();
    for (method, rows) in HELP {
        let group = adw::PreferencesGroup::builder().title(*method).build();
        for (effect, keys) in *rows {
            let row = adw::ActionRow::builder().title(gettext(*effect)).build();
            row.add_suffix(&keycaps(keys));
            group.add(&row);
        }
        list.append(&group);
        groups.push(group);

        let pill = gtk::Button::builder()
            .label(*method)
            .css_classes(["pill"])
            .build();
        pills.insert(&pill, -1);
        buttons.push(pill);
    }

    // A pill filters to its section; clicking the active pill again shows all.
    let groups = Rc::new(groups);
    let buttons = Rc::new(buttons);
    let active: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    for (i, pill) in buttons.iter().enumerate() {
        pill.connect_clicked({
            let groups = groups.clone();
            let buttons = buttons.clone();
            let active = active.clone();
            move |_| {
                let selected = if *active.borrow() == Some(i) {
                    None
                } else {
                    Some(i)
                };
                *active.borrow_mut() = selected;
                for (j, group) in groups.iter().enumerate() {
                    group.set_visible(selected.is_none() || selected == Some(j));
                }
                for (j, button) in buttons.iter().enumerate() {
                    if selected == Some(j) {
                        button.add_css_class("suggested-action");
                    } else {
                        button.remove_css_class("suggested-action");
                    }
                }
            }
        });
    }

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.append(&pills);
    outer.append(&scrolled);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&outer));
    window.set_content(Some(&toolbar));
    window.present();
}

/// A row of keycaps for a `" + "`-separated key string (e.g. `"Vowel + S"`),
/// with a dimmed `+` between them. The `Vowel` placeholder is translatable.
fn keycaps(keys: &str) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .valign(gtk::Align::Center)
        .build();
    for (i, token) in keys.split(" + ").enumerate() {
        if i > 0 {
            let plus = gtk::Label::new(Some("+"));
            plus.add_css_class("dim-label");
            row.append(&plus);
        }
        let text = if token == "Vowel" {
            gettext("Vowel")
        } else {
            token.to_string()
        };
        let label = gtk::Label::builder()
            .label(text)
            .css_classes(["bambusa-key", "monospace"])
            .build();
        row.append(&label);
    }
    row
}

/// Install the keycap style once, on the default display.
fn install_keycap_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        ".bambusa-key { \
         border: 1px solid alpha(currentColor, 0.25); \
         border-radius: 6px; \
         padding: 1px 7px; \
         min-width: 12px; \
         }",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
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
            let cb_window = window.clone();
            dialog.open(Some(&window), gtk::gio::Cancellable::NONE, move |res| {
                // res is Err when the user cancels the picker — nothing to do.
                let Ok(file) = res else { return };
                let Some(path) = file.path() else { return };
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(_) => {
                        show_import_error(&cb_window, &gettext("Could not read the file."));
                        return;
                    }
                };
                match parse_macro_file(&content) {
                    Ok(pairs) => {
                        for (key, value) in pairs {
                            add_row(&key, &value);
                        }
                    }
                    Err(message) => show_import_error(&cb_window, &message),
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

/// Validate and parse a macro file into `(shortcut, expansion)` pairs. Blank
/// lines and `#`/`;` comments are ignored; every other line must be a
/// `shortcut:expansion` pair with both parts non-empty. Returns a
/// human-readable error for the first malformed line, or if no macros are found.
fn parse_macro_file(content: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
            continue;
        }
        match s.split_once(':') {
            Some((key, value)) if !key.trim().is_empty() && !value.trim().is_empty() => {
                pairs.push((key.trim().to_string(), value.trim().to_string()));
            }
            _ => {
                return Err(format!(
                    "{}\n\n{} {}: {s}",
                    gettext(
                        "This file is not a valid macro list — each line must be in the “shortcut:expansion” format."
                    ),
                    gettext("Line"),
                    index + 1,
                ));
            }
        }
    }
    if pairs.is_empty() {
        return Err(gettext("The file does not contain any macros."));
    }
    Ok(pairs)
}

/// Show a modal error dialog for a failed macro import.
fn show_import_error(parent: &impl IsA<gtk::Window>, message: &str) {
    let heading = gettext("Import failed");
    let dialog = adw::MessageDialog::new(Some(parent), Some(heading.as_str()), Some(message));
    dialog.add_response("ok", &gettext("OK"));
    dialog.present();
}

/// The display label for a charset internal name (most are shown verbatim; the
/// Vietnamese-named ones get a translatable label).
fn charset_label(name: &str) -> String {
    if name == "Unicode tổ hợp" {
        gettext("Composed Unicode")
    } else {
        name.to_string()
    }
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

#[cfg(test)]
mod tests {
    use super::parse_macro_file;

    #[test]
    fn parses_a_valid_file() {
        let content = "# a comment\n; another\n\nvn:Việt Nam\nbtw:  by the way \n";
        let pairs = parse_macro_file(content).unwrap();
        assert_eq!(
            pairs,
            vec![
                ("vn".to_string(), "Việt Nam".to_string()),
                ("btw".to_string(), "by the way".to_string()),
            ]
        );
    }

    #[test]
    fn rejects_a_line_without_a_colon() {
        assert!(parse_macro_file("vn:ok\nnot a macro line\n").is_err());
    }

    #[test]
    fn rejects_empty_shortcut_or_expansion() {
        assert!(parse_macro_file(":value").is_err());
        assert!(parse_macro_file("key:").is_err());
    }

    #[test]
    fn rejects_a_file_with_no_macros() {
        assert!(parse_macro_file("# only comments\n\n").is_err());
    }
}
