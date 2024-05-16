TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os

# BOARD
BOARD ?= qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# Default target
.PHONY: all
all: kernel

# Build the kernel in release mode
.PHONY: kernel
kernel:
	cargo build --release

# Build the kernel in debug mode
.PHONY: kernel-debug
kernel-debug:
	cargo build

# Clean the build artifacts
.PHONY: clean
clean:
	cargo clean

# Run the kernel using QEMU
.PHONY: run
run: kernel
	timeout --foreground 30s qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-kernel $(KERNEL_ELF)

# Debug the kernel using QEMU and GDB
.PHONY: gdb
gdb: kernel
	qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -kernel $(KERNEL_ELF) -s -S

# Build the kernel in debug mode and run it with QEMU
.PHONY: debug
debug: MODE := debug
debug: KERNEL_ELF := target/$(TARGET)/$(MODE)/os
debug: kernel-debug
	qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -kernel $(KERNEL_ELF) -s -S