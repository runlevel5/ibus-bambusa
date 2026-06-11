# COPR "make_srpm" build method.
#
# COPR invokes:  make -f .copr/Makefile srpm outdir=<resultdir> spec=<specpath>
# (cwd is the repo root). We download the source tarball and the checksum that
# release CI published on the GitHub Release, verify, then build the SRPM. No
# lookaside cache and no manual tarball upload are involved.

# Note: COPR invokes this Makefile with `spec=<path>` on the command line, which
# would override a `spec :=` here, so we use an uppercase SPEC it does not touch.
SPEC    := build/rpm/ibus-bambusa.spec
topdir  := $(CURDIR)/_build
baseurl := https://github.com/runlevel5/ibus-bambusa/releases/download

srpm:
	dnf -y install rpm-build rpmdevtools
	mkdir -p $(topdir)/SOURCES
	spectool -g -R --define "_topdir $(topdir)" $(SPEC)
	# Fetch the .sha256 CI published next to the tarball and verify the download.
	version=$$(rpmspec -q --srpm --qf '%{version}' $(SPEC)); \
	tarball="ibus-bambusa-$$version.tar.gz"; \
	curl -fsSL -o "$(topdir)/SOURCES/$$tarball.sha256" "$(baseurl)/v$$version/$$tarball.sha256"; \
	cd $(topdir)/SOURCES && sha256sum -c "$$tarball.sha256"
	rpmbuild -bs --define "_topdir $(topdir)" $(SPEC)
	cp $(topdir)/SRPMS/*.src.rpm $(outdir)

.PHONY: srpm
