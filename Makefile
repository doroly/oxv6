# Target architecture triple for bare-metal RISC-V 64-bit
TARGET := riscv64gc-unknown-none-elf

# Path to the compiled kernel binary
KERNEL_ELF := target/$(TARGET)/debug/oxv6

# Number of CPU cores (harts) to simulate
CPUS ?= 3

# QEMU emulator binary
QEMU := qemu-system-riscv64

# QEMU configuration flags
QEMUFLAGS := -machine virt
QEMUFLAGS += -nographic
QEMUFLAGS += -bios default
QEMUFLAGS += -smp $(CPUS)
QEMUFLAGS += -kernel $(KERNEL_ELF)

.PHONY: all build run clean qemu

# Default target builds the kernel
all: build

# Compile the kernel binary using Cargo
build:
	cargo build

# Build and execute the kernel in QEMU with multi-hart support
run: build
	$(QEMU) $(QEMUFLAGS)

# Remove build artifacts and temporary files
clean:
	cargo clean

# Perform a clean rebuild and immediately start QEMU
qemu: clean run