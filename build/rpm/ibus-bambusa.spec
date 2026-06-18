Name:           ibus-bambusa
Version:        0.4.0
Release:        1%{?dist}
Summary:        Vietnamese input method engine for GNOME (Wayland)

License:        GPL-3.0-or-later
URL:            https://github.com/runlevel5/ibus-bambusa
# Release CI attaches this tarball (and a matching .sha256) to the GitHub
# Release for the version tag. It unpacks to a name-version dir that autosetup
# expects. See .github/workflows/release.yml and .copr/Makefile.
Source0:        %{url}/releases/download/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  meson >= 0.61
BuildRequires:  gcc
# Provides %%cargo_prep / %%cargo_generate_buildrequires; pulls in cargo + rust.
# The Rust dependencies are taken from Fedora's packaged crates (vendored in
# /usr/share/cargo/registry) rather than fetched from crates.io.
BuildRequires:  cargo-rpm-macros >= 26
BuildRequires:  gettext
BuildRequires:  desktop-file-utils
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  pkgconfig(glib-2.0)

Requires:       ibus
Requires:       hicolor-icon-theme

%description
ibus-bambusa is a Vietnamese input method engine for IBus, targeting GNOME on
Wayland. It supports the Telex, VNI and VIQR typing methods, every common output
charset, spelling validation, text macros and a libadwaita preferences GUI.

%prep
%autosetup
# Point cargo at the system crate registry (/usr/share/cargo/registry), build
# offline, and drop the upstream Cargo.lock so deps resolve to Fedora's crates.
%cargo_prep

# Emit BuildRequires for our crate dependencies from Cargo.toml; RPM dependency
# resolution then pulls the full transitive rust-*-devel set.
%generate_buildrequires
%cargo_generate_buildrequires

%build
# cargo-build.sh keys off this to build --offline (no --locked) against the
# registry %%cargo_prep configured, instead of fetching the pinned lock.
export CARGO_NET_OFFLINE=true
%meson
%meson_build

%install
%meson_install
%find_lang %{name}

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/ibus-setup-bambusa.desktop

%files -f %{name}.lang
%license LICENSE
%doc README.md
%dir %{_libexecdir}/ibus-bambusa
%{_libexecdir}/ibus-bambusa/ibus-engine-bambusa
%{_libexecdir}/ibus-bambusa/ibus-setup-bambusa
%{_datadir}/ibus/component/bambusa.xml
%dir %{_datadir}/ibus-bambusa
%dir %{_datadir}/ibus-bambusa/icons
%{_datadir}/ibus-bambusa/icons/vi.svg
%{_datadir}/ibus-bambusa/vietnamese.cm.dict
%{_datadir}/ibus-bambusa/LICENSE.vietnamese.cm.dict
%{_datadir}/applications/ibus-setup-bambusa.desktop
%{_datadir}/applications/org.freedesktop.IBus.bambusa.setup.desktop
%{_datadir}/glib-2.0/schemas/org.freedesktop.IBus.bambusa.gschema.xml

%changelog
* Sun Jun 14 2026 Trung Lê <8@tle.id.au> - 0.4.0-1
- Unregister engine objects on destroy (fixes a per-context object-server leak)
  and cap concurrently live engines.

* Sun Jun 14 2026 Trung Lê <8@tle.id.au> - 0.3.0-1
- Reduce per-keystroke allocations in the compose engine (zero-copy rule
  lookup, scratch-buffer reuse, pre-sized buffers).

* Fri Jun 12 2026 Trung Lê <8@tle.id.au> - 0.2.0-1
- Add libadwaita preferences GUI and its setup desktop entry.
- Add dictionary-based spell-check and text macros.
- Add Vietnamese translation.

* Tue Jun 09 2026 Trung Lê <8@tle.id.au> - 0.1.0-1
- Initial package.
