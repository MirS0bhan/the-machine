# The Machine — Build System

CARGO      ?= cargo
QEMU       ?= qemu-system-x86_64
KERNEL     ?= $(shell ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1)
INITRAMFS  ?= build/initramfs.cpio.gz
ISO        ?= build/the-machine.iso
MEM        ?= 1G
PYTHON     ?= python3

.PHONY: all build build-release initramfs initramfs-release iso iso-release qemu run run-console clean \
        test test-rust test-python test-all verify docs help \
        services-start services-stop ci-package

all: build initramfs iso docs

build:
	$(CARGO) build --workspace

build-release:
	$(CARGO) build --workspace --release

# Assemble the bootable initramfs (busybox + our Rust services).
initramfs:
	bash build/mkinitramfs.sh debug

initramfs-release:
	bash build/mkinitramfs.sh release

# Build a bootable ISO (GRUB) that loads the kernel + initramfs.
iso: initramfs
	bash build/mkiso.sh "$(KERNEL)" "$(INITRAMFS)" "$(ISO)"

iso-release: initramfs-release
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

test: test-all

test-rust:
	$(CARGO) test --workspace

test-python:
	$(PYTHON) -m pytest tests/integration/ -v
	$(PYTHON) -m pytest ui-engine/test_engine.py -v
	$(PYTHON) -m pytest ui-engine-demo/test_demo.py -v
	$(PYTHON) -m pytest policy-broker/tests/ -v
	$(PYTHON) -m pytest state-store/tests/ -v
	$(PYTHON) -m pytest local-model/tests/ -v
	cd lambda-server && $(PYTHON) test_server.py

test-all: test-rust test-python

verify:
	bash scripts/verify-all.sh

docs:
	$(MAKE) -C docs html

services-start:
	bash scripts/start-services.sh

services-stop:
	bash scripts/stop-services.sh

# Package per-component builds + ISO for CI (run after initramfs-release + iso).
ci-package: build-release initramfs-release iso-release
	bash build/ci-package-artifacts.sh build/artifacts release

help:
	@echo "The Machine — Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all            - Build crates, initramfs, ISO, and docs"
	@echo "  build          - Build all Rust crates"
	@echo "  build-release  - Build release binaries"
	@echo "  initramfs      - Assemble the initramfs (busybox + services)"
	@echo "  iso            - Build the bootable ISO image"
	@echo "  qemu           - Boot kernel+initramfs directly in QEMU"
	@echo "  run            - Boot the ISO in QEMU"
	@echo "  test           - Run all tests (Rust + Python)"
	@echo "  verify         - Full verification (tests + builds + docs + inventory)"
	@echo "  test-rust      - Run Rust workspace tests"
	@echo "  test-python    - Run Python unit + integration tests"
	@echo "  docs           - Build documentation"
	@echo "  services-start - Start all services (dev harness)"
	@echo "  services-stop  - Stop dev harness services"
	@echo "  ci-package     - Build release + ISO + artifact bundle"
	@echo "  clean          - Remove build artifacts"
