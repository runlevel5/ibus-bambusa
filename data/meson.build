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
