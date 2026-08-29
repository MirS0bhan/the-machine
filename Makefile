# The Machine — Build System

CARGO      ?= cargo
QEMU       ?= qemu-system-x86_64
KERNEL     ?= /boot/vmlinuz-7.1.8
INITRAMFS  ?= build/initramfs.cpio.gz
ISO        ?= build/the-machine.iso
MEM        ?= 1G

.PHONY: all build build-release initramfs iso qemu run run-console clean test docs help

all: build iso docs

build:
	$(CARGO) build --workspace

build-release:
	$(CARGO) build --workspace --release

# Assemble the bootable initramfs (busybox + our Rust services).
initramfs:
	bash build/mkinitramfs.sh debug

# Build a bootable ISO (GRUB) that loads the kernel + initramfs.
iso: initramfs
	bash build/mkiso.sh "$(KERNEL)" "$(INITRAMFS)" "$(ISO)"

# Boot directly from the kernel + initramfs (fast iteration, no ISO).
qemu: initramfs
	$(QEMU) -enable-kvm -kernel "$(KERNEL)" -initrd "$(INITRAMFS)" \
		-append "console=ttyS0,115200 rdinit=/init" -m $(MEM) -nographic

# Boot the produced ISO in QEMU.
run: iso
	$(QEMU) -enable-kvm -m $(MEM) -cdrom "$(ISO)" -nographic

run-console: iso
	$(QEMU) -enable-kvm -m $(MEM) -cdrom "$(ISO)" -nographic

clean:
	$(CARGO) clean
	rm -rf build/initramfs.stage build/initramfs.cpio.gz build/the-machine.iso build/iso

test:
	$(CARGO) test --workspace

docs:
	make -C docs html

help:
	@echo "The Machine — Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all          - Build crates, ISO, and docs"
	@echo "  build        - Build all Rust crates"
	@echo "  build-release- Build release binaries"
	@echo "  initramfs    - Assemble the initramfs (busybox + services)"
	@echo "  iso          - Build the bootable ISO image"
	@echo "  qemu         - Boot kernel+initramfs directly in QEMU"
	@echo "  run          - Boot the ISO in QEMU"
	@echo "  run-console  - Same as run, serial console to stdout"
	@echo "  clean        - Remove build artifacts"
	@echo "  test         - Run tests"
	@echo "  docs         - Build documentation"
