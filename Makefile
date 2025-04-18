TARGET_ARCH=aarch64-unknown-linux-gnu
ROOTNAME=target/$(TARGET_ARCH)/release/pipin
ROOTNAME_DEBUG=target/$(TARGET_ARCH)/debug/pipin

REMOTE_HOST=pi08@192.168.68.68
REMOTE_DIR=~/pipin/

build:
	cargo build

run:
	cargo run

cross-build:
	cargo build --target $(TARGET_ARCH)

release:
	cargo build --release --target $(TARGET_ARCH)

qemu: cross-build
	qemu-aarch64 -L /usr/aarch64-linux-gnu $(ROOTNAME_DEBUG)

qemu-release: release
	qemu-aarch64 -L /usr/aarch64-linux-gnu $(ROOTNAME)

clean: 
	rm -rf hardware/*.a hardware/*.o && cargo clean

remote: release
	rsync -az $(ROOTNAME) $(REMOTE_HOST):$(REMOTE_DIR)/

watch:
	systemfd --no-pid -s http::3000 -- cargo watch -w src/ -w frontend/ -x run
