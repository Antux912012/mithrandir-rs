.PHONY: all build test check clean deb flatpak publish

all: build deb flatpak

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

flatpak:
	./packaging/build_flatpak.sh

publish:
	./packaging/publish_github.sh

clean:
	cargo clean
	rm -rf build-dir-rs repo-rs target/pkg-deb .flatpak-builder
