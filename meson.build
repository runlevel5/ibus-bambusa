project(
    'ibus-bambusa',
    version: '0.1.0',
    license: 'MIT',
    meson_version: '>= 0.61.0',
)

cargo = find_program('cargo')
cargo_build = find_program('build-aux/cargo-build.sh')

prefix = get_option('prefix')
libexecdir = prefix / get_option('libexecdir')
datadir = prefix / get_option('datadir')

profile = get_option('buildtype') == 'debug' ? 'debug' : 'release'

# Build the engine binary with cargo (which honours rust-toolchain.toml).
custom_target(
    'ibus-engine-bambusa',
    output: 'ibus-engine-bambusa',
    build_by_default: true,
    build_always_stale: true,
    console: true,
    install: true,
    install_dir: libexecdir / 'ibus-bambusa',
    command: [
        cargo_build,
        meson.project_source_root(),
        meson.project_build_root() / 'cargo-target',
        '@OUTPUT@',
        profile,
        'ibus-engine-bambusa',
    ],
)

subdir('data')
