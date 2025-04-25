TARGET_ARCH=aarch64-unknown-linux-musl
ROOTNAME=target/$(TARGET_ARCH)/release/pipin
ROOTNAME_DEBUG=target/$(TARGET_ARCH)/debug/pipin

REMOTE_HOST=pi08@192.168.68.70
REMOTE_DIR=~/pipin/

# https://stackoverflow.com/a/31778003/17123405

build:
	cargo build

run:
	cargo run

cross-build:
	cargo build --target $(TARGET_ARCH)

release:
	cargo build --release --target $(TARGET_ARCH)

qemu: cross-build
	qemu-aarch64 -L /usr/aarch64-linux-musl $(ROOTNAME_DEBUG)

qemu-release: release
	qemu-aarch64 -L /usr/aarch64-linux-musl $(ROOTNAME)

clean: 
	cargo clean

remote: release
	rsync -az $(ROOTNAME) $(REMOTE_HOST):$(REMOTE_DIR)/

watch:
	systemfd --no-pid -s http::3000 -- cargo watch -w src/ -w frontend/ -x run
