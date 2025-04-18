TARGET_ARCH=aarch64-unknown-linux-gnu
ROOTNAME=target/$(TARGET_ARCH)/release/pipinrs

REMOTE_HOST=pi08@192.168.68.68
REMOTE_DIR=~/pipinrs/

VERSION ?= $(shell git describe --tags --always)
ZIPNAME = pipinrs-$(VERSION).zip

build:
	cargo build

run:
	cargo run

cross-build:
	cargo build --target $(TARGET_ARCH)

release:
	cargo build --release --target $(TARGET_ARCH)

qemu-test: cross-build
	qemu-aarch64 -L /usr/aarch64-linux-gnu target/$(TARGET_ARCH)/debug/pipinrs

qemu-release-test: release
	qemu-aarch64 -L /usr/aarch64-linux-gnu target/$(TARGET_ARCH)/release/pipinrs

clean: 
	rm -rf hardware/*.a hardware/*.o && cargo clean

remote: release
	rsync -az $(ROOTNAME) $(REMOTE_HOST):$(REMOTE_DIR)/

watch:
	systemfd --no-pid -s http::3000 -- cargo watch -w src/ -w frontend/ -x run

release-zip: release
	mkdir -p releases
	cd target/$(TARGET_ARCH)/release && zip ../../../releases/$(ZIPNAME) pipinrs
	@echo "created releases/$(ZIPNAME)"

publish-release: release-zip
	gh release create $(VERSION) --title "Release $(VERSION)" --notes "Release $(VERSION)" releases/$(ZIPNAME)
