SHELL := /bin/sh
.SHELLFLAGS := -eu -c
.DEFAULT_GOAL := build

# ---------------------------------------------------------------------------
# 主机环境探测 (兼容任意发行版 / 内核版本 / 架构)
# ---------------------------------------------------------------------------
UNAME_S        := $(shell uname -s 2>/dev/null)
UNAME_M        := $(shell uname -m 2>/dev/null)
KERNEL_RELEASE := $(shell uname -r 2>/dev/null)
DISTRO_ID      := $(shell . /etc/os-release 2>/dev/null && echo "$$ID")
DISTRO_VER     := $(shell . /etc/os-release 2>/dev/null && echo "$$VERSION_ID")

ifneq ($(UNAME_S),Linux)
$(error 本项目仅支持 Linux 主机, 当前: $(UNAME_S))
endif

# ---------------------------------------------------------------------------
# 必要工具链 — 任何发行版都必须满足
# ---------------------------------------------------------------------------
REQUIRED_TOOLS := bash clang cargo rustc pkg-config

# ---------------------------------------------------------------------------
# 包管理器探测 — 用于无 BTF 时自动安装 pahole (dwarves)
# ---------------------------------------------------------------------------
PKG_MANAGER := $(shell \
    if   command -v apt-get >/dev/null 2>&1; then echo apt; \
    elif command -v dnf     >/dev/null 2>&1; then echo dnf; \
    elif command -v yum     >/dev/null 2>&1; then echo yum; \
    elif command -v pacman  >/dev/null 2>&1; then echo pacman; \
    elif command -v zypper  >/dev/null 2>&1; then echo zypper; \
    elif command -v apk     >/dev/null 2>&1; then echo apk; \
    else echo unknown; fi)

# 非 root 时使用 sudo
SUDO := $(shell [ "$$(id -u)" != "0" ] && command -v sudo >/dev/null 2>&1 && echo sudo)

# ---------------------------------------------------------------------------
# BTF / vmlinux 相关
# ---------------------------------------------------------------------------
BTF_VMLINUX := /sys/kernel/btf/vmlinux
INCLUDE_DIR := $(CURDIR)/scripts/include
VMLINUX_H   := $(INCLUDE_DIR)/vmlinux.h
VMLINUX_BTF := $(INCLUDE_DIR)/vmlinux.btf

# 常见 vmlinux (含调试符号) 路径候选
VMLINUX_CANDIDATES := \
    /usr/lib/debug/lib/modules/$(KERNEL_RELEASE)/vmlinux \
    /usr/lib/debug/usr/lib/modules/$(KERNEL_RELEASE)/vmlinux \
    /usr/lib/debug/boot/vmlinux-$(KERNEL_RELEASE) \
    /usr/lib/debug/boot/vmlinux-$(KERNEL_RELEASE).debug \
    /usr/lib/modules/$(KERNEL_RELEASE)/vmlinux \
    /boot/vmlinux-$(KERNEL_RELEASE)

# 解析时探测 BTF (兼容低版本内核没有 /sys/kernel/btf 节点)
HAS_BTF := $(shell [ -r $(BTF_VMLINUX) ] && echo yes || echo no)

ifeq ($(HAS_BTF),yes)
CARGO_ENV   :=
EXTRA_TOOLS :=
BTF_DEPS    :=
else
CARGO_ENV   := OPENTRACE_BPF_INCLUDE=$(INCLUDE_DIR)
EXTRA_TOOLS := pahole
BTF_DEPS    := install-pahole
endif

# ---------------------------------------------------------------------------
# Targets
# ---------------------------------------------------------------------------
.PHONY: build release info check-tools check-btf vmlinux install-pahole install-debuginfo clean help deny

build: info $(BTF_DEPS) check-tools check-btf
	@echo ">>> cargo build  (HAS_BTF=$(HAS_BTF), arch=$(UNAME_M))"
	$(CARGO_ENV) cargo build

release: info $(BTF_DEPS) check-tools check-btf
	@echo ">>> cargo build --release  (HAS_BTF=$(HAS_BTF), arch=$(UNAME_M))"
	$(CARGO_ENV) cargo build --release

# 跟 .github/workflows/cargo-deny.yaml 行为对齐：跳过 licenses 检查。
# 想跑完整检查（含 license）用 `cargo deny check`。
deny:
	cargo deny check advisories bans sources

info:
	@echo ">>> Distro : $(DISTRO_ID) $(DISTRO_VER)"
	@echo ">>> Kernel : $(KERNEL_RELEASE)"
	@echo ">>> Arch   : $(UNAME_M)"
	@echo ">>> BTF    : $(HAS_BTF)"

# 1. 工具链检测 — 一次性列出所有缺失项
check-tools:
	@missing=""; \
	for tool in $(REQUIRED_TOOLS) $(EXTRA_TOOLS); do \
	    command -v "$$tool" >/dev/null 2>&1 || missing="$$missing $$tool"; \
	done; \
	if [ -n "$$missing" ]; then \
	    echo "错误: 缺少工具:$$missing" >&2; \
	    echo "提示: pahole 来自 dwarves 包; rust 工具链建议用 rustup 安装" >&2; \
	    exit 1; \
	fi; \
	echo ">>> 工具链 OK: $(REQUIRED_TOOLS) $(EXTRA_TOOLS)"

# 2. BTF 检查；不支持则生成 vmlinux.{h,btf}
check-btf:
ifeq ($(HAS_BTF),yes)
	@echo ">>> 内核已开放 BTF: $(BTF_VMLINUX)"
else
	@echo ">>> 内核未开放 BTF, 生成本地 vmlinux.{h,btf}"
	@$(MAKE) --no-print-directory vmlinux
endif

vmlinux: $(VMLINUX_H) $(VMLINUX_BTF)

$(VMLINUX_H) $(VMLINUX_BTF): | install-pahole install-debuginfo
	@mkdir -p $(INCLUDE_DIR)
	@set -e; \
	echo ">>> pahole 版本: $$(pahole --version 2>&1 | head -n1)"; \
	vmlinux_path=""; \
	for path in $(VMLINUX_CANDIDATES); do \
	    if [ -f "$$path" ]; then vmlinux_path="$$path"; break; fi; \
	done; \
	if [ -z "$$vmlinux_path" ]; then \
	    echo "错误: 未在常见路径找到 vmlinux 调试文件, 请确认 debuginfo 包已安装" >&2; \
	    exit 1; \
	fi; \
	echo ">>> 使用 vmlinux: $$vmlinux_path"; \
	pahole --btf_encode_detached $(VMLINUX_BTF) "$$vmlinux_path"; \
	echo ">>> 已生成 BTF 文件: $(VMLINUX_BTF)"; \
	command -v bpftool >/dev/null 2>&1 || { echo "错误: 未找到 bpftool, 请安装 (apt: linux-tools-common / dnf: bpftool / pacman: bpf)" >&2; exit 1; }; \
	bpftool btf dump file $(VMLINUX_BTF) format c > $(VMLINUX_H); \
	echo ">>> 已生成头文件 (bpftool): $(VMLINUX_H)"
	# 原 pahole 生成方式 (已弃用, 改用 bpftool btf dump):
	# pahole --compile "$$vmlinux_path" > $(VMLINUX_H); \
	# echo ">>> 已生成头文件: $(VMLINUX_H)"

# 3. 无 BTF 时自动安装 pahole (dwarves)
install-pahole:
	@if command -v pahole >/dev/null 2>&1; then \
	    echo ">>> pahole 已安装: $$(command -v pahole)"; \
	else \
	    echo ">>> 内核未开放 BTF, 正在通过 $(PKG_MANAGER) 安装 pahole (dwarves)..."; \
	    case "$(PKG_MANAGER)" in \
	        apt)    $(SUDO) apt-get update && $(SUDO) apt-get install -y dwarves ;; \
	        dnf)    $(SUDO) dnf install -y dwarves ;; \
	        yum)    $(SUDO) yum install -y dwarves ;; \
	        pacman) $(SUDO) pacman -S --noconfirm pahole ;; \
	        zypper) $(SUDO) zypper install -y dwarves ;; \
	        apk)    $(SUDO) apk add dwarves ;; \
	        *)      echo "错误: 未识别的包管理器, 请手动安装 dwarves/pahole 包" >&2; exit 1 ;; \
	    esac; \
	    command -v pahole >/dev/null 2>&1 || { echo "错误: pahole 安装失败" >&2; exit 1; }; \
	    echo ">>> pahole 安装完成: $$(command -v pahole)"; \
	fi

# 4. 安装当前内核对应的 debuginfo / dbgsym (各发行版策略不同)
install-debuginfo:
	@echo ">>> 安装 debuginfo (kernel=$(KERNEL_RELEASE), pm=$(PKG_MANAGER))"
	@set -e; \
	case "$(PKG_MANAGER)" in \
	    apt) \
	        codename="$$(. /etc/os-release && echo "$${VERSION_CODENAME:-}")"; \
	        list=/etc/apt/sources.list.d/ddebs.list; \
	        if [ ! -f "$$list" ] && [ -n "$$codename" ] && grep -qi ubuntu /etc/os-release; then \
	            echo ">>> 启用 Ubuntu ddebs 仓库"; \
	            printf '%s\n' \
	                "deb http://ddebs.ubuntu.com $$codename main restricted universe multiverse" \
	                "deb http://ddebs.ubuntu.com $$codename-updates main restricted universe multiverse" \
	                "deb http://ddebs.ubuntu.com $$codename-proposed main restricted universe multiverse" \
	                | $(SUDO) tee "$$list" >/dev/null; \
	            $(SUDO) apt-get install -y ubuntu-dbgsym-keyring 2>/dev/null \
	                || $(SUDO) apt-key adv --keyserver keyserver.ubuntu.com \
	                       --recv-keys F2EDC64DC5AEE1F6B9C621F0C8CAB6595FDFF622 \
	                || true; \
	        fi; \
	        $(SUDO) apt-get update; \
	        $(SUDO) apt-get install -y "linux-image-$(KERNEL_RELEASE)-dbgsym" \
	        ;; \
	    dnf) \
	        $(SUDO) dnf install -y dnf-plugins-core || true; \
	        $(SUDO) dnf debuginfo-install -y "kernel-$(KERNEL_RELEASE)" \
	            || $(SUDO) dnf install -y "kernel-debuginfo-$(KERNEL_RELEASE)" \
	        ;; \
	    yum) \
	        $(SUDO) yum install -y yum-utils || true; \
	        if command -v debuginfo-install >/dev/null 2>&1; then \
	            $(SUDO) debuginfo-install -y "kernel-$(KERNEL_RELEASE)"; \
	        else \
	            $(SUDO) yum install -y "kernel-debuginfo-$(KERNEL_RELEASE)"; \
	        fi \
	        ;; \
	    zypper) \
	        kr="$(KERNEL_RELEASE)"; \
	        $(SUDO) zypper --non-interactive install "kernel-default-debuginfo=$${kr%-default}" \
	        ;; \
	    apk) \
	        case "$(KERNEL_RELEASE)" in \
	            *-virt) flavor="linux-virt-dbg" ;; \
	            *-lts)  flavor="linux-lts-dbg"  ;; \
	            *-edge) flavor="linux-edge-dbg" ;; \
	            *)      flavor="linux-lts-dbg"  ;; \
	        esac; \
	        $(SUDO) apk update; \
	        $(SUDO) apk add "$$flavor" \
	        ;; \
	    pacman) \
	        $(SUDO) pacman -Sy --noconfirm linux-headers; \
	        echo "提示: Arch 官方仓库不提供 kernel-debuginfo, linux-headers 已安装。如需 vmlinux 调试符号请从 AUR (linux-debug) 安装。" \
	        ;; \
	    *) \
	        echo "错误: 未识别的包管理器, 请手动安装 kernel debuginfo" >&2; \
	        exit 1 \
	        ;; \
	esac; \
	echo ">>> debuginfo 安装完成"

clean:
	cargo clean
	rm -f $(VMLINUX_H) $(VMLINUX_BTF)

help:
	@echo "Targets:"
	@echo "  build              debug 编译 (默认)"
	@echo "  release            release 编译"
	@echo "  info               打印主机信息 (发行版/内核/架构/BTF)"
	@echo "  check-tools        检查必备工具 (无 BTF 时附加 pahole)"
	@echo "  check-btf          检查 BTF, 不支持则生成 vmlinux.{h,btf}"
	@echo "  install-pahole     无 BTF 时自动安装 pahole (dwarves)"
	@echo "  install-debuginfo  安装当前内核对应的 debuginfo / dbgsym 包"
	@echo "  vmlinux            生成 vmlinux.{h,btf} 到 scripts/include/"
	@echo "  clean              cargo clean + 删除生成产物"
