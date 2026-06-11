Name:           ibus-bambusa
Version:        0.2.0
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
BuildRequires:  cargo >= 1.96.0
BuildRequires:  rust >= 1.96.0
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

%build
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
* Fri Jun 12 2026 Trung Lê <8@tle.id.au> - 0.2.0-1
- Add libadwaita preferences GUI and its setup desktop entry.
- Add dictionary-based spell-check and text macros.
- Add Vietnamese translation.

* Tue Jun 09 2026 Trung Lê <8@tle.id.au> - 0.1.0-1
- Initial package.
