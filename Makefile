ROOTNAME=target/aarch64-unknown-linux-musl/release/gpio_controller

REMOTE_HOST=pi08@192.168.68.70
#REMOTE_HOST=pi08@raspberrypi08
REMOTE_DIR=~/gpio_controller/

build:
	cargo build

release:
	cargo build --release

clean: 
	cargo clean

run:
	cargo run

remote: 
	rsync -az $(ROOTNAME) $(REMOTE_HOST):$(REMOTE_DIR)/
