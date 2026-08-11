# oxv6: Rust RISC-V Operating System
[English](README.md) | [中文](README_ZH.md)

`oxv6` is a Unix-like operating system kernel written in Rust, inspired by MIT's **xv6-riscv**. It targets the **RISC-V 64 (riscv64gc)** architecture and runs on the **QEMU `virt`** machine platform.

---

## Project Overview

- **Target Architecture**: RISC-V 64-bit (`riscv64gc-unknown-none-elf`)
- **Reference Model**: MIT xv6-riscv
- **Language**: Rust (`no_std` bare-metal environment)
- **Environment**: Linux (Ubuntu 20.04 LTS recommended) / QEMU `virt`

## Implemented Features

1. **UART Serial Driver**
    - Implements basic UART 16550 output driver supporting string formatting and hexadecimal address printing.
2. **Physical Memory Allocator (Frame Allocator)**
    - Manages 4 KiB physical pages using an intrusive singly-linked list (zero memory overhead).
    - Features memory poisoning (garbage filling) for debugging dangling pointers and uninitialized memory reads.

---

## Environment Setup (Ubuntu 20.04)

Follow these steps to set up the build and emulation environment on Ubuntu 20.04 LTS.

### 1. Install System Dependencies & QEMU

```bash
sudo apt update
sudo apt install -y build-essential curl git qemu-system-misc gdb-multiarch
```

### 2. Install Rust Toolchain

If Rust is not installed on your system, install it via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
source $HOME/.cargo/env
```

### 3. Configure Nightly Toolchain & Target

Bare-metal OS development requires the Rust `nightly` toolchain and the RISC-V cross-compilation target:

```bash
# Install and set nightly toolchain
rustup toolchain install nightly
rustup default nightly

# Add RISC-V 64 target
rustup target add riscv64gc-unknown-none-elf
```

---

## Building and Running

### 1. Launch via QEMU

Run the kernel directly using `make`:

```bash
make run
```

*(Or invoke `qemu-system-riscv64` manually if runner is not configured in `.cargo/config.toml`)*:

```bash
qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel target/riscv64gc-unknown-none-elf/debug/oxv6
```

### 2. Exit QEMU

Because QEMU runs in non-graphical mode (`-nographic`), terminal I/O is attached directly to the UART serial line.

To terminate the simulation:

1. Press `Ctrl + A`
2. Release, then press `X`
