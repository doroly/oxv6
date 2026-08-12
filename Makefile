.PHONY: build run clean qemu

build:
	cargo build

run: build
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios default \
		-kernel target/riscv64gc-unknown-none-elf/debug/oxv6

clean:
	cargo clean

qemu: clean run