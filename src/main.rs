#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

// 1. 內聯彙編：初始化棧指針 (SP)，然後跳轉到 Rust 入口函數
global_asm!(
    r#"
    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_top
    call rust_main

loop:
    j loop

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 4  # 分配 16KB 啟動棧
    .globl boot_stack_top
boot_stack_top:
"#
);

// 2. 裸機環境必需的 Panic 處理
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// 3. 通過 QEMU virtual 板卡的 MMIO UART (0x10000000) 輸出字符
fn putchar(ch: u8) {
    let uart = 0x1000_0000 as *mut u8;
    unsafe {
        uart.write_volatile(ch);
    }
}

// 讀取一個字符 (阻塞模式)
fn getchar() -> u8 {
    let uart_base = 0x1000_0000 as *mut u8;
    // LSR 暫存器位於偏移 0x5
    let lsr = unsafe { uart_base.add(5) };

    // 循環檢查直到第 0 位為 1 (Data Ready)
    while unsafe { lsr.read_volatile() & 1 == 0 } {
        // 這裡可以加入 CPU 指令 nop 或簡單延遲，避免過度耗能，但目前這樣即可
    }

    // 從偏移 0 的位置讀取字符
    unsafe { uart_base.read_volatile() }
}


fn print_str(s: &str) {
    for byte in s.bytes() {
        putchar(byte);
    }
}

// 4. Rust 核心主入口
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    print_str("Hello, RVOS from Rust!\n");

    loop {
        let ch = getchar();

        // 特殊字符處理：Enter (回車)
        if ch == b'\r' {
            putchar(b'\r');
            putchar(b'\n'); // 換行
        } else {
            // 普通字符直接回顯
            putchar(ch);
        }
    }
}
