ROOTNAME=target/aarch64-unknown-linux-musl/release/pipinrs

REMOTE_HOST=pi08@192.168.68.68
#REMOTE_HOST=pi08@raspberrypi08
REMOTE_DIR=~/pipinrs/

build:
	cargo build

release:
	cargo build --release 

clean: 
	rm -rf hardware/*.a hardware/*.o && cargo clean

remote: release
	rsync -az $(ROOTNAME) $(REMOTE_HOST):$(REMOTE_DIR)/

local:
	cargo build --target x86_64-unknown-linux-gnu

run:
	cargo run --target x86_64-unknown-linux-gnu

watch:
	systemfd --no-pid -s http::3000 -- cargo watch -w src/ -w frontend/ -x "run --target x86_64-unknown-linux-gnu" 
