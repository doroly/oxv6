.PHONY: build run clean

build:
	cargo build

run: build
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel target/riscv64gc-unknown-none-elf/debug/RVOS

clean:
	cargo clean