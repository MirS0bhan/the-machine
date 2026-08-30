#!/usr/bin/env bash
# Shared rootfs helpers (G13 — kernel + layout for target HW installs).

rootfs_skeleton_dirs() {
  local rootfs="$1"
  mkdir -p "${rootfs}"/{bin,sbin,etc,boot,the-machine,var/lib/the-machine,usr/lib/systemd/system}
}

rootfs_has_debootstrap() {
  [[ -x "${1}/usr/bin/apt-get" ]]
}

# Install a Debian kernel package inside a debootstrap rootfs (requires sudo + network).
rootfs_install_kernel_debian() {
  local rootfs="$1"
  if ! rootfs_has_debootstrap "${rootfs}"; then
    return 1
  fi

  echo "==> Installing linux-image-amd64 in rootfs chroot"
  mount --bind /dev "${rootfs}/dev"
  mount --bind /proc "${rootfs}/proc"
  mount --bind /sys "${rootfs}/sys"
  # shellcheck disable=SC2064
  trap 'umount "${rootfs}/dev" "${rootfs}/proc" "${rootfs}/sys" 2>/dev/null || true' RETURN

  chroot "${rootfs}" apt-get update -qq
  DEBIAN_FRONTEND=noninteractive chroot "${rootfs}" apt-get install -y -qq linux-image-amd64
}

# Expose /vmlinuz at the root for grub.cfg (installer expects it).
rootfs_link_vmlinuz() {
  local rootfs="$1"
  mkdir -p "${rootfs}/boot"

  local vmlinuz
  vmlinuz="$(find "${rootfs}/boot" -maxdepth 1 -name 'vmlinuz-*' -type f 2>/dev/null | sort | tail -1 || true)"
  if [[ -n "${vmlinuz}" ]]; then
    ln -sfn "$(basename "${vmlinuz}")" "${rootfs}/boot/vmlinuz"
    ln -sfn "boot/vmlinuz" "${rootfs}/vmlinuz"
    echo "==> Kernel linked at ${rootfs}/vmlinuz"
    return 0
  fi

  local host="/boot/vmlinuz-$(uname -r)"
  if [[ -f "${host}" ]]; then
    local dest="vmlinuz-$(uname -r)"
    install -m 0644 "${host}" "${rootfs}/boot/${dest}"
    ln -sfn "${dest}" "${rootfs}/boot/vmlinuz"
    ln -sfn "boot/vmlinuz" "${rootfs}/vmlinuz"
    echo "WARN: copied host kernel into rootfs (skeleton/dev build)" >&2
    return 0
  fi

  echo "WARN: no kernel in rootfs — debootstrap + linux-image-amd64 required on target HW (G13)" >&2
  return 1
}
