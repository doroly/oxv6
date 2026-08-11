/* 指定目標二進制檔案的架構為 RISC-V */
OUTPUT_ARCH(riscv)

/* 指定程式執行的第一條指令位置（入口符號）為 _start */
ENTRY(_start)

/* 定義 RISC-V QEMU virt 板子加載 Kernel 的基底實體地址 */
BASE_ADDRESS = 0x80200000;

SECTIONS {
    /* 將當前內存位址計數器（Location Counter .）設為基底地址 0x80200000 */
    . = BASE_ADDRESS;

    /* .text 程式碼段 */
    .text : {
        *(.text.entry) /* 強制將標記為 .text.entry 的代碼（即 _start 彙編）放在最開頭 */
        *(.text .text.*) /* 放入其餘所有 Rust 函數的程式碼 */
    }

    /* .rodata 只讀數據段 (Read-Only Data) */
    .rodata : {
        *(.rodata .rodata.*) /* 存放字串字面量、const 常數等 */
    }

    /* .data 已初始化的全域與靜態變數段 */
    .data : {
        *(.data .data.*) /* 存放有初始值的 static 變數 */
    }

    /* .bss 未初始化的全域變數段 (Block Started by Symbol) */
    .bss : {
        *(.bss .bss.*) /* 存放未初始化或預設為 0 的變數（如啟動內存棧） */
    }

    /* /DISCARD/ 丟棄段 */
    /DISCARD/ : {
        *(.eh_frame) /* 丟棄 C++/Rust 異常展開 (Unwind) 相關的調試段，減少內存佔用並避免裸機編譯錯誤 */
    }
}