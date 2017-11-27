MEMORY
{
    FLASH(RX) : ORIGIN = 0x08000000, LENGTH = 1024K
    RAM(WAIL) : ORIGIN = 0x20000000, LENGTH = 320K
}

ENTRY(reset)

__DATA_LOAD = LOADADDR(.data);

SECTIONS
{
    .text : ALIGN(4)
    {
        /* stack pointer */
        LONG(ORIGIN(RAM) + LENGTH(RAM))
        /* reset entry point */
        LONG(reset + 1)

        KEEP(*(.rodata.EXCEPTIONS));
        KEEP(*(.rodata.INTERRUPTS));

        *(.text*)
    } > FLASH

    /* Constant data goes into FLASH */
    .rodata :
    {
      *(.rodata .rodata*)
    } >FLASH

    .ARM.extab   : { *(.ARM.extab* .gnu.linkonce.armextab.*) } >FLASH

    .data : ALIGN(4)
    {
      __DATA_START = .;
      *(.data*)
      . = ALIGN(4);
      __DATA_END = .;
    } > RAM AT > FLASH

    /* Uninitialized data section */
    . = ALIGN(4);
    .bss :
    {
      /* This is used by the startup in order to initialize the .bss secion */
      __BSS_START = .;         /* define a global symbol at bss start */
      *(.bss)
      *(.bss*)
      *(COMMON)

      . = ALIGN(4);
      __BSS_END = .;         /* define a global symbol at bss end */
    } >RAM

    /DISCARD/ :
    {
      *(.ARM.exidx*)
    }

    __HEAP_START = .;
    . += 50K;
    __HEAP_END = .;

    __STACK_START = ORIGIN(RAM) + LENGTH(RAM);
}
