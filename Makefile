.PHONY: all build test check clean deb rpm flatpak aur publish

all: build deb rpm flatpak aur

build:
	flatpak run --user --devel --share=network --filesystem=home --command=sh org.gnome.Sdk//50 -c \
	  "export PATH=\$$HOME/.cargo/bin:\$$PATH && cargo build --release --features gui"

check:
	cargo check
	flatpak run --user --devel --share=network --filesystem=home --command=sh org.gnome.Sdk//50 -c \
	  "export PATH=\$$HOME/.cargo/bin:\$$PATH && cargo check --features gui"

test:
	cargo test

deb:
	./packaging/build_deb.sh

rpm:
	./packaging/build_rpm.sh

flatpak:
	./packaging/build_flatpak.sh

aur:
	@echo "==> AUR PKGBUILD is ready in packaging/aur/PKGBUILD"

publish:
	./packaging/publish_github.sh

clean:
	cargo clean
	rm -rf build-dir-rs repo-rs target/pkg-deb target/pkg-rpm target/pkg-flatpak target/repo-flatpak .flatpak-builder
