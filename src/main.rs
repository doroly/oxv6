// 禁用 Rust 標準庫 (std)，因為裸機 OS 沒有作業系統底層 API
#![no_std]
// 禁用標準的 main 函數入口，改由我們定義的 _start 引導
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

// 嵌入引導彙編代碼：進行硬體棧空間配置並跳轉至 Rust 入口
global_asm!(
    r#"
    .section .text.entry          # 指定將此段代碼放入 .text.entry 區塊（對應 kernel.ld 最開頭）
    .globl _start                 # 導出 _start 符號，讓鏈接器能夠找到入口
_start:
    la sp, boot_stack_top         # 加載內存棧頂地址到 sp (Stack Pointer) 暫存器
    call rust_main                # 呼叫 Rust 主函數 rust_main

loop:
    j loop                        # 若 rust_main 意外返回，在此死循環

    .section .bss.stack           # 將棧空間放入 .bss 段
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 4               # 在內存中預留 16 KB (4 個 Page) 作為啟動棧空間
    .globl boot_stack_top
boot_stack_top:                   # 棧頂標記（RISC-V 棧是由高地址向低地址增長）
"#
);

/// 裸機環境下的 Panic 處理器
/// 當程式發生 panic 時會自動呼叫此函數
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // 發生嚴重錯誤時直接進入死循環
    loop {}
}

/// 通過 16550 UART 串口發送一個字元到控制台 (MMIO)
fn putchar(ch: u8) {
    // QEMU virt 板的 UART 映射基底地址
    let uart = 0x1000_0000 as *mut u8;
    unsafe {
        // 揮發性寫入，防止編譯器優化掉硬體操作
        uart.write_volatile(ch);
    }
}

/// 從控制台讀取一個字元 (阻塞模式)
fn getchar() -> u8 {
    // QEMU virt 板的 UART 映射基底地址
    let uart = 0x1000_0000 as *mut u8;
    // LSR (Line Status Register) 位於偏移 5，第 0 位代表是否有數據可讀 (Data Ready)
    // 輪詢等待，直到硬體接收到按鍵輸入
    while unsafe { uart.add(5).read_volatile() & 1 == 0 } {}
    // 從偏移 0 (RBR) 讀取字元
    unsafe { uart.read_volatile() }
}

/// 印出字串工具函數
fn print_str(s: &str) {
    for byte in s.bytes() {
        putchar(byte);
    }
}

// 保持符號名稱為 rust_main，禁止 Rust 進行名稱重命名 (Mangle)
#[unsafe(no_mangle)]
// Rust 邏輯主入口，使用 C 呼叫約定，永不返回
pub extern "C" fn rust_main() -> ! {
    print_str("Hello, RVOS from Rust!\n");
    print_str("RVOS Console Active. Type something:\n");

    loop {
        // 阻塞等待輸入
        let ch = getchar();

        // 特殊字元處理：Enter 鍵處理
        if ch == b'\r' {
            putchar(b'\r');
            // 終端按下 Enter 會發送 '\r'，需手動補上 '\n' 實現換行並歸位
            putchar(b'\n');
        } else {
            // 一般字元直接回顯 (Echo) 到螢幕
            putchar(ch);
        }
    }
}
