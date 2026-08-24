#!/usr/bin/env bash
# Mini AI 1C — Setup dependencies for Linux/macOS
# Usage: ./scripts/setup.sh

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_err() { echo -e "${RED}[ERR]${NC} $*"; }

OS="$(uname -s)"
ARCH="$(uname -m)"

check_cmd() { command -v "$1" >/dev/null 2>&1; }

install_rust() {
    if check_cmd cargo; then
        log_info "Rust уже установлен: $(cargo --version)"
        return
    fi
    log_info "Установка Rust через rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    rustup component add rustfmt clippy
    log_info "Rust установлен: $(cargo --version)"
}

install_node() {
    if check_cmd node && check_cmd npm; then
        log_info "Node.js уже установлен: $(node --version), npm: $(npm --version)"
        return
    fi
    case "$OS" in
        Linux)
            if check_cmd apt-get; then
                log_info "Установка Node.js через apt..."
                curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
                sudo apt-get install -y nodejs
            elif check_cmd dnf; then
                log_info "Установка Node.js через dnf..."
                sudo dnf module install -y nodejs:20
            elif check_cmd pacman; then
                log_info "Установка Node.js через pacman..."
                sudo pacman -S --noconfirm nodejs npm
            else
                log_err "Неизвестный пакетный менеджер. Установите Node.js 20+ вручную."
                exit 1
            fi
            ;;
        Darwin)
            if check_cmd brew; then
                log_info "Установка Node.js через Homebrew..."
                brew install node@20
            else
                log_err "Homebrew не найден. Установите Node.js 20+ вручную с https://nodejs.org"
                exit 1
            fi
            ;;
    esac
    log_info "Node.js: $(node --version), npm: $(npm --version)"
}

install_java() {
    if check_cmd java; then
        JAVA_VER=$(java -version 2>&1 | head -1 | grep -oE '[0-9]+' | head -1)
        if [[ "$JAVA_VER" -ge 17 ]]; then
            log_info "Java уже установлена: $(java -version 2>&1 | head -1)"
            return
        fi
        log_warn "Java версия < 17, требуется 17+"
    fi
    case "$OS" in
        Linux)
            if check_cmd apt-get; then
                log_info "Установка Java 17 через apt..."
                sudo apt-get update && sudo apt-get install -y openjdk-17-jdk
            elif check_cmd dnf; then
                log_info "Установка Java 17 через dnf..."
                sudo dnf install -y java-17-openjdk-devel
            elif check_cmd pacman; then
                log_info "Установка Java 17 через pacman..."
                sudo pacman -S --noconfirm jdk17-openjdk
            else
                log_err "Установите OpenJDK 17+ вручную."
                exit 1
            fi
            ;;
        Darwin)
            if check_cmd brew; then
                log_info "Установка Java 17 через Homebrew..."
                brew install openjdk@17
                sudo ln -sfn "$(brew --prefix)/opt/openjdk@17/libexec/openjdk.jdk" /Library/Java/JavaVirtualMachines/openjdk-17.jdk
            else
                log_err "Homebrew не найден. Установите OpenJDK 17+ вручную."
                exit 1
            fi
            ;;
    esac
    log_info "Java: $(java -version 2>&1 | head -1)"
}

install_system_deps_linux() {
    log_info "Установка системных зависимостей для Linux..."
    if check_cmd apt-get; then
        sudo apt-get update
        sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            patchelf \
            pkg-config \
            build-essential \
            curl \
            wget \
            git \
            sqlite3 \
            libssl-dev
    elif check_cmd dnf; then
        sudo dnf install -y \
            webkit2gtk4.1-devel \
            gtk3-devel \
            libayatana-appindicator-gtk3-devel \
            librsvg2-devel \
            patchelf \
            pkgconfig \
            gcc-c++ \
            make \
            curl \
            wget \
            git \
            sqlite \
            openssl-devel
    elif check_cmd pacman; then
        sudo pacman -S --noconfirm \
            webkit2gtk-4.1 \
            gtk3 \
            libayatana-appindicator \
            librsvg \
            patchelf \
            pkgconf \
            base-devel \
            curl \
            wget \
            git \
            sqlite \
            openssl
    else
        log_warn "Неизвестный дистрибутив. Установите зависимости вручную:"
        log_warn "webkit2gtk, gtk3, libayatana-appindicator, librsvg, patchelf, pkg-config, build tools"
    fi
}

install_system_deps_macos() {
    log_info "Установка системных зависимостей для macOS..."
    if check_cmd brew; then
        brew install pkg-config
    else
        log_warn "Homebrew не найден. Установите pkg-config вручную."
    fi
}

install_dotnet() {
    if check_cmd dotnet; then
        DOTNET_VER=$(dotnet --version | cut -d. -f1)
        if [[ "$DOTNET_VER" -ge 8 ]]; then
            log_info ".NET уже установлен: $(dotnet --version)"
            return
        fi
        log_warn ".NET версия < 8, требуется 8+"
    fi
    case "$OS" in
        Linux)
            if check_cmd apt-get; then
                log_info "Установка .NET 8 через Microsoft репозиторий..."
                wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb -O packages-microsoft-prod.deb
                sudo dpkg -i packages-microsoft-prod.deb
                rm packages-microsoft-prod.deb
                sudo apt-get update && sudo apt-get install -y dotnet-sdk-8.0
            elif check_cmd dnf; then
                log_info "Установка .NET 8 через dnf..."
                sudo dnf install -y dotnet-sdk-8.0
            elif check_cmd pacman; then
                log_info "Установка .NET 8 через pacman..."
                sudo pacman -S --noconfirm dotnet-sdk
            else
                log_err "Установите .NET 8 SDK вручную: https://dotnet.microsoft.com/download"
                exit 1
            fi
            ;;
        Darwin)
            if check_cmd brew; then
                log_info "Установка .NET 8 через Homebrew..."
                brew install --cask dotnet-sdk
            else
                log_err "Homebrew не найден. Установите .NET 8 SDK вручную."
                exit 1
            fi
            ;;
    esac
    log_info ".NET: $(dotnet --version)"
}

install_tauri_cli() {
    if check_cmd cargo-tauri; then
        log_info "Tauri CLI уже установлен: $(cargo tauri --version 2>/dev/null || echo 'ok')"
        return
    fi
    log_info "Установка Tauri CLI..."
    cargo install tauri-cli --version "^2.0"
    log_info "Tauri CLI установлен"
}

main() {
    log_info "=== Mini AI 1C Setup ==="
    log_info "OS: $OS, ARCH: $ARCH"

    case "$OS" in
        Linux)
            install_system_deps_linux
            install_rust
            install_node
            install_java
            install_dotnet
            install_tauri_cli
            ;;
        Darwin)
            install_system_deps_macos
            install_rust
            install_node
            install_java
            install_dotnet
            install_tauri_cli
            ;;
        *)
            log_err "Неподдерживаемая ОС: $OS"
            exit 1
            ;;
    esac

    log_info "=== Готово! ==="
    log_info "Перезапустите терминал или выполните: source ~/.cargo/env"
    log_info "Затем: cd tauri-app && npm install && npm run build:mcp"
}

main "$@"