.PHONY: deb clean

deb:
	cargo deb
	PKG_CONFIG=$(shell which pkg-config) PKG_CONFIG_PATH=/lib/i386-linux-gnu/pkgconfig cargo deb --target i686-unknown-linux-gnu

clean:
	rm -rf target

