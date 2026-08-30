#!/usr/bin/env bash
# End-to-end boot greet user story (no QEMU required).
#
# User story:
# 1. ISO boots services (compositor → ui-runtime → agent)
# 2. boot.auil lays out greeting + chat widgets
# 3. boot.system.ready wakes agent → boot.greet plan patches chat UI
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "==> boot greet e2e: boot.auil layout"
grep -q 'ui\.greeting' build/boot.auil
grep -q 'ui\.chat_log' build/boot.auil
grep -q 'ui\.chat_input' build/boot.auil
grep -q 'agent\.chat\.send' build/boot.auil

echo "==> boot greet e2e: AUIL parser"
cargo test -p ui-runtime auil::tests::boot_auil_does_not_insert_root_under_itself --quiet
cargo test -p ui-runtime auil::tests::parses_simple_stack --quiet

echo "==> boot greet e2e: agent planner"
cargo test -p agent-core planner::tests::boot_greet_plan_updates_chat_ui --quiet
cargo test -p agent-core planner::tests::chat_message_plan_appends_user_line --quiet

echo "==> boot greet e2e: compositor text + framebuffer dump"
THE_MACHINE_COMPOSITOR_BACKEND=memory \
THE_MACHINE_FB_DUMP=/tmp/boot-greet-frame.ppm \
  cargo test -p compositor bitmap_font::tests::draws_hello_without_panic --quiet
THE_MACHINE_COMPOSITOR_BACKEND=memory \
THE_MACHINE_FB_DUMP=/tmp/boot-greet-frame.ppm \
  cargo test -p compositor pixel::tests::memory_buffer_paints_pixels --quiet
test -s /tmp/boot-greet-frame.ppm

echo "==> boot greet e2e passed"
