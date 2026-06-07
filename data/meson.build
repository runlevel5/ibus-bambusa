conf = configuration_data()
conf.set('libexecdir', libexecdir)
conf.set('datadir', datadir)
conf.set('version', meson.project_version())

# Generate the IBus component descriptor with install paths substituted in.
configure_file(
    input: 'bambusa.xml.in',
    output: 'bambusa.xml',
    configuration: conf,
    install: true,
    install_dir: datadir / 'ibus' / 'component',
)

install_data(
    'icons/vi.svg',
    install_dir: datadir / 'ibus-bambusa' / 'icons',
)

# Word dictionary for dictionary-based spell-check (and its upstream license).
install_data(
    'vietnamese.cm.dict',
    'LICENSE.vietnamese.cm.dict',
    install_dir: datadir / 'ibus-bambusa',
)

# Desktop entry GNOME Settings launches for the engine "Preferences" button
# (it looks up ibus-setup-<engine-family>.desktop, i.e. ibus-setup-bambusa).
configure_file(
    input: 'ibus-setup-bambusa.desktop.in',
    output: 'ibus-setup-bambusa.desktop',
    configuration: conf,
    install: true,
    install_dir: datadir / 'applications',
)

# GSettings schema: install it and (re)compile the schema cache.
gnome = import('gnome')
install_data(
    'org.freedesktop.IBus.bambusa.gschema.xml',
    install_dir: datadir / 'glib-2.0' / 'schemas',
)
gnome.post_install(glib_compile_schemas: true)
# Also compile into the build dir so the engine/GUI can run uninstalled
# (GSETTINGS_SCHEMA_DIR pointing at this build directory).
gnome.compile_schemas(
    depend_files: files('org.freedesktop.IBus.bambusa.gschema.xml'),
)
