# The Machine — Build System

CARGO      ?= cargo
QEMU       ?= qemu-system-x86_64
KERNEL     ?= $(shell ls -1 /boot/vmlinuz-* 2>/dev/null | sort -V | tail -1)
INITRAMFS  ?= build/initramfs.cpio.gz
ISO        ?= build/the-machine.iso
MEM        ?= 1G
PYTHON     ?= python3

.PHONY: all build build-release initramfs initramfs-release iso iso-release qemu run run-console clean \
        test test-rust test-python test-all test-build-scripts verify verify-docs coverage lint docs help \
        services-start services-stop ci-package fetch-model rootfs rootfs-release

all: build initramfs iso docs

build:
	$(CARGO) build --workspace

build-release:
	$(CARGO) build --workspace --release

# Assemble the bootable initramfs (busybox + our Rust services).
fetch-model:
	bash build/fetch-model.sh

initramfs: fetch-model
	bash build/mkinitramfs.sh debug

initramfs-release: fetch-model
	bash build/mkinitramfs.sh release

# Build a bootable ISO (GRUB) that loads the kernel + initramfs.
iso: initramfs
	bash build/mkiso.sh "$(KERNEL)" "$(INITRAMFS)" "$(ISO)"

iso-release: initramfs-release
	bash build/mkiso.sh "$(KERNEL)" "$(INITRAMFS)" "$(ISO)"

# KVM when available (bare metal / nested virt); TCG otherwise (CI, cloud VMs).
QEMU_ACCEL ?= $(shell if [ -r /dev/kvm ]; then echo -enable-kvm; else echo -accel tcg; fi)

# Boot directly from the kernel + initramfs (fast iteration, no ISO).
qemu: initramfs
	$(QEMU) $(QEMU_ACCEL) -kernel "$(KERNEL)" -initrd "$(INITRAMFS)" \
		-append "console=ttyS0,115200 rdinit=/init" -m $(MEM) -nographic

# Boot the produced ISO in QEMU.
# `run` uses a graphical display when $DISPLAY is set (framebuffer compositor);
# otherwise it falls back to serial. `run-console` is always nographic.
run: iso
	@if [ -n "$$DISPLAY" ]; then \
		$(QEMU) $(QEMU_ACCEL) -m $(MEM) -cdrom "$(ISO)" -vga std; \
	else \
		$(QEMU) $(QEMU_ACCEL) -m $(MEM) -cdrom "$(ISO)" -nographic; \
	fi

run-console: iso
	$(QEMU) $(QEMU_ACCEL) -m $(MEM) -cdrom "$(ISO)" -nographic

clean:
	$(CARGO) clean
	rm -rf build/initramfs.stage build/initramfs.cpio.gz build/the-machine.iso build/iso

test: test-all

test-build-scripts:
	bash build/test-assemble-release.sh

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

test-all: test-rust test-python test-build-scripts

verify:
	bash scripts/verify-all.sh

verify-docs:
	bash scripts/verify-docs-code.sh

lint:
	$(CARGO) fmt -p common -p mcp-bus -p system-daemon -p fallback-shell -- --check
	$(CARGO) clippy -p common -p mcp-bus -p system-daemon -p fallback-shell -- -D warnings -A dead_code

coverage:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || (echo "Installing cargo-llvm-cov..." && cargo install cargo-llvm-cov)
	cargo llvm-cov --workspace --lcov --output-path build/coverage.lcov
	@echo "Coverage report: build/coverage.lcov"

docs:
	$(MAKE) -C docs html

services-start:
	bash scripts/start-services.sh

services-stop:
	bash scripts/stop-services.sh

# Package per-component builds + ISO for CI (run after initramfs-release + iso).
rootfs:
	bash build/mkrootfs.sh minimal

rootfs-release:
	bash build/mkrootfs.sh release

install-help:
	@echo "Run: sudo bash build/installer/install.sh /dev/sdX"

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
	@echo "  run            - Boot the ISO in QEMU (graphical if DISPLAY is set)"
	@echo "  run-console    - Boot the ISO in QEMU (serial / nographic)"
	@echo "  lint           - rustfmt + clippy on socket/bus/daemon crates"
	@echo "  fetch-model    - Download or stub the GGUF weights for the ISO"
	@echo "  rootfs         - Build a minimal rootfs (see build/mkrootfs.sh)"
	@echo "  test           - Run all tests (Rust + Python + build scripts)"
	@echo "  test-build-scripts - Release assemble regression (CI rust-* glob)"
	@echo "  verify         - Full verification (tests + builds + docs + inventory)"
	@echo "  verify-docs    - Cross-check docs against component-inventory.yaml"
	@echo "  coverage       - Rust test coverage (llvm-cov → build/coverage.lcov)"
	@echo "  test-rust      - Run Rust workspace tests"
	@echo "  test-python    - Run Python unit + integration tests"
	@echo "  docs           - Build documentation"
	@echo "  services-start - Start all services (dev harness)"
	@echo "  services-stop  - Stop dev harness services"
	@echo "  ci-package     - Build release + ISO + artifact bundle"
	@echo "  clean          - Remove build artifacts"
