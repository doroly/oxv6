# oxv6：Rust RISC-V 作業系統
[English](README.md) | [中文](README_ZH.md)

`oxv6` 是一個使用 Rust 語言開發的 Unix-like 作業系統核心，設計靈感與架構參考 MIT 的 **xv6-riscv**。本專案針對 **RISC-V 64 (riscv64gc)** 架構設計，並運行於 **QEMU `virt`** 虛擬機平台。

---

## 專案簡介

- **目標架構**：RISC-V 64位元 (`riscv64gc-unknown-none-elf`)
- **參考實現**：MIT xv6-riscv
- **開發語言**：Rust (`no_std` 裸機開發環境)
- **運行環境**：Linux (推薦 Ubuntu 20.04 LTS) / QEMU `virt`


## 已實現功能

1. **UART 串口驅動**
    - 實現基礎 UART 16550 驅動，支援字串與十六進位位址列印。
2. **實體記憶體分配器 (Frame Allocator)**
    - 基於隱式單向鏈表管理 4 KiB 頁面，零額外記憶體開銷。
    - 支援記憶體毒化（垃圾數據填充），方便捕獲懸空指標與讀取未初始化記憶體錯誤。

---

## 開發環境安裝 (Ubuntu 20.04)

請在 Ubuntu 20.04 LTS 環境下按以下步驟配置編譯工具鏈與模擬器：

### 1. 安裝系統套件與 QEMU

```bash
sudo apt update
sudo apt install -y build-essential curl git qemu-system-misc gdb-multiarch
```

### 2. 安裝 Rust 工具鏈

若系統尚未安裝 Rust，請透過 `rustup` 進行安裝：

```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh
source $HOME/.cargo/env
```

### 3. 設定 Nightly 工具鏈與編譯目標

作業系統裸機開發需要使用 Rust `nightly` 工具鏈與 RISC-V 交叉編譯目標：

```bash
# 安裝並切換至 nightly 工具鏈
rustup toolchain install nightly
rustup default nightly

# 新增 RISC-V 64 裸機目標架構
rustup target add riscv64gc-unknown-none-elf
```

---

## 編譯與運行方式

### 1. 執行項目

在專案根目錄下直接使用 `make` 啟動：

```bash
make qemu
```

### 2. 退出 QEMU 模擬器

專案運行於無圖形介面模式 (`-nographic`)，終端會由 UART 串口接管。

**退出快捷鍵**：

1. 按下組合鍵 `Ctrl + A`
2. 鬆開按鍵，緊接著按下 `X` 鍵
