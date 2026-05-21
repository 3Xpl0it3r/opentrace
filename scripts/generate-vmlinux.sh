#!/usr/bin/env bash
# 安装当前内核对应的 debuginfo / dbgsym 包，用于 BPF 项目获取 BTF / vmlinux 等调试信息。
# 支持 apt (Debian/Ubuntu)、dnf/yum (RHEL/CentOS/Fedora/Amazon Linux)、
# zypper (openSUSE/SLES)、apk (Alpine)、pacman (Arch)。

set -euo pipefail

KERNEL_RELEASE="$(uname -r)"

if [ "$(id -u)" -ne 0 ]; then
    SUDO="sudo"
else
    SUDO=""
fi

detect_pkg_manager() {
    if command -v apt-get >/dev/null 2>&1; then
        echo "apt"
    elif command -v dnf >/dev/null 2>&1; then
        echo "dnf"
    elif command -v yum >/dev/null 2>&1; then
        echo "yum"
    elif command -v zypper >/dev/null 2>&1; then
        echo "zypper"
    elif command -v apk >/dev/null 2>&1; then
        echo "apk"
    elif command -v pacman >/dev/null 2>&1; then
        echo "pacman"
    else
        echo "unknown"
    fi
}

install_apt() {
    # Ubuntu/Debian 的内核 dbgsym 在 ddebs 仓库；如果没启用先尝试启用。
    local codename
    codename="$(. /etc/os-release && echo "${VERSION_CODENAME:-}")"
    local list="/etc/apt/sources.list.d/ddebs.list"

    if [ ! -f "$list" ] && [ -n "$codename" ] && grep -qi ubuntu /etc/os-release; then
        echo ">>> 启用 Ubuntu ddebs 仓库"
        $SUDO tee "$list" >/dev/null <<EOF
deb http://ddebs.ubuntu.com ${codename} main restricted universe multiverse
deb http://ddebs.ubuntu.com ${codename}-updates main restricted universe multiverse
deb http://ddebs.ubuntu.com ${codename}-proposed main restricted universe multiverse
EOF
        $SUDO apt-get install -y ubuntu-dbgsym-keyring || \
            $SUDO apt-key adv --keyserver keyserver.ubuntu.com \
                  --recv-keys F2EDC64DC5AEE1F6B9C621F0C8CAB6595FDFF622 || true
    fi

    $SUDO apt-get update
    $SUDO apt-get install -y "linux-image-${KERNEL_RELEASE}-dbgsym"
}

install_dnf() {
    $SUDO dnf install -y dnf-plugins-core || true
    $SUDO dnf debuginfo-install -y "kernel-${KERNEL_RELEASE}" || \
        $SUDO dnf install -y "kernel-debuginfo-${KERNEL_RELEASE}"
}

install_yum() {
    # 部分发行版需先启用 debuginfo 仓库
    $SUDO yum install -y yum-utils || true
    if command -v debuginfo-install >/dev/null 2>&1; then
        $SUDO debuginfo-install -y "kernel-${KERNEL_RELEASE}"
    else
        $SUDO yum install -y "kernel-debuginfo-${KERNEL_RELEASE}"
    fi
}

install_zypper() {
    $SUDO zypper --non-interactive install "kernel-default-debuginfo=${KERNEL_RELEASE%-default}"
}

install_apk() {
    # Alpine 通过内核 flavor 区分 (-virt / -lts / -edge)，从 uname 推断
    local flavor
    case "$KERNEL_RELEASE" in
        *-virt)  flavor="linux-virt-dbg" ;;
        *-lts)   flavor="linux-lts-dbg" ;;
        *-edge)  flavor="linux-edge-dbg" ;;
        *)       flavor="linux-lts-dbg" ;;
    esac
    $SUDO apk update
    $SUDO apk add "$flavor"
}

install_pacman() {
    $SUDO pacman -Sy --noconfirm linux-headers
    echo "提示: Arch 官方仓库不提供 kernel-debuginfo，linux-headers 已安装。" \
         "如需 vmlinux 调试符号请从 AUR (linux-debug) 安装。"
}

# 在常见路径里查找当前内核对应的带调试符号的 vmlinux 文件
locate_vmlinux() {
    local candidates=(
        # RHEL / CentOS / Fedora / 麒麟 / 欧拉
        "/usr/lib/debug/lib/modules/${KERNEL_RELEASE}/vmlinux"
        "/usr/lib/debug/usr/lib/modules/${KERNEL_RELEASE}/vmlinux"
        # Debian / Ubuntu
        "/usr/lib/debug/boot/vmlinux-${KERNEL_RELEASE}"
        # openSUSE / SLES
        "/usr/lib/debug/boot/vmlinux-${KERNEL_RELEASE}.debug"
        # Alpine / Arch (内核包内嵌符号)
        "/usr/lib/modules/${KERNEL_RELEASE}/vmlinux"
        "/boot/vmlinux-${KERNEL_RELEASE}"
    )

    for path in "${candidates[@]}"; do
        if [ -f "$path" ]; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

generate_btf() {
    if ! command -v pahole >/dev/null 2>&1; then
        echo "错误: 未找到 pahole, 请先安装 (apt: dwarves / yum: dwarves / apk: dwarves)。" >&2
        exit 1
    fi
    echo ">>> pahole 版本: $(pahole --version 2>&1 | head -n1)"

    local vmlinux_path
    if ! vmlinux_path="$(locate_vmlinux)"; then
        echo "错误: 未在常见路径找到 vmlinux 调试文件, 请确认 debuginfo 包已安装。" >&2
        exit 1
    fi
    echo ">>> 使用 vmlinux: ${vmlinux_path}"

    local btf_out="$(pwd)/vmlinux.btf"
    pahole --btf_encode_detached "$btf_out" "$vmlinux_path"
    echo ">>> 已生成 BTF 文件: ${btf_out}"

    local header_out="$(pwd)/vmlinux.h"
    pahole --compile "$vmlinux_path" > "$header_out"
    echo ">>> 已生成头文件: ${header_out}"
}

main() {
    local pm
    pm="$(detect_pkg_manager)"
    echo ">>> 内核版本: ${KERNEL_RELEASE}"
    echo ">>> 检测到包管理器: ${pm}"

    case "$pm" in
        apt)     install_apt ;;
        dnf)     install_dnf ;;
        yum)     install_yum ;;
        zypper)  install_zypper ;;
        apk)     install_apk ;;
        pacman)  install_pacman ;;
        *)
            echo "错误: 未识别的包管理器, 请手动安装 kernel debuginfo。" >&2
            exit 1
            ;;
    esac

    echo ">>> debuginfo 安装完成"
    generate_btf
}

main "$@"
