#!/usr/bin/env node
// Mini AI 1C — Universal setup script (Node.js)
// Usage: node scripts/setup.mjs
// Or add to package.json: "setup": "node scripts/setup.mjs"

import { execSync } from 'node:child_process';
import { platform, arch } from 'node:os';

const OS = platform();
const ARCH = arch();

const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
};

function log(level, msg) {
  const prefix = { info: 'INFO', warn: 'WARN', err: 'ERR' }[level];
  const color = { info: colors.green, warn: colors.yellow, err: colors.red }[level];
  console.log(`${color}[${prefix}]${colors.reset} ${msg}`);
}

function hasCmd(cmd) {
  try {
    execSync(`command -v ${cmd}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

function hasCmdWin(cmd) {
  try {
    execSync(`where ${cmd}`, { stdio: 'ignore', windowsHide: true });
    return true;
  } catch {
    return false;
  }
}

function run(cmd, opts = {}) {
  log('info', `$ ${cmd}`);
  execSync(cmd, { stdio: 'inherit', ...opts });
}

async function setupLinux() {
  log('info', 'Обнаружен Linux');
  const pm = hasCmd('apt-get') ? 'apt' : hasCmd('dnf') ? 'dnf' : hasCmd('pacman') ? 'pacman' : null;
  if (!pm) throw new Error('Неподдерживаемый пакетный менеджер');

  // System deps
  const pkgs = {
    apt: [
      'libwebkit2gtk-4.1-dev', 'libgtk-3-dev', 'libayatana-appindicator3-dev',
      'librsvg2-dev', 'patchelf', 'pkg-config', 'build-essential',
      'curl', 'wget', 'git', 'sqlite3', 'libssl-dev',
    ],
    dnf: [
      'webkit2gtk4.1-devel', 'gtk3-devel', 'libayatana-appindicator-gtk3-devel',
      'librsvg2-devel', 'patchelf', 'pkgconfig', 'gcc-c++', 'make',
      'curl', 'wget', 'git', 'sqlite', 'openssl-devel',
    ],
    pacman: [
      'webkit2gtk-4.1', 'gtk3', 'libayatana-appindicator', 'librsvg',
      'patchelf', 'pkgconf', 'base-devel', 'curl', 'wget', 'git', 'sqlite', 'openssl',
    ],
  };
  const installCmd = {
    apt: `sudo apt-get update && sudo apt-get install -y ${pkgs.apt.join(' ')}`,
    dnf: `sudo dnf install -y ${pkgs.dnf.join(' ')}`,
    pacman: `sudo pacman -S --noconfirm ${pkgs.pacman.join(' ')}`,
  };
  run(installCmd[pm]);

  // Rust
  if (!hasCmd('cargo')) {
    run('curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y');
    process.env.PATH += ':' + process.env.HOME + '/.cargo/bin';
    run('rustup component add rustfmt clippy');
  }

  // Node.js
  if (!hasCmd('node')) {
    if (pm === 'apt') {
      run('curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -');
      run('sudo apt-get install -y nodejs');
    } else if (pm === 'dnf') {
      run('sudo dnf module install -y nodejs:20');
    } else if (pm === 'pacman') {
      run('sudo pacman -S --noconfirm nodejs npm');
    }
  }

  // Java 17
  if (!hasCmd('java')) {
    if (pm === 'apt') run('sudo apt-get install -y openjdk-17-jdk');
    else if (pm === 'dnf') run('sudo dnf install -y java-17-openjdk-devel');
    else if (pm === 'pacman') run('sudo pacman -S --noconfirm jdk17-openjdk');
  }

  // .NET 8
  if (!hasCmd('dotnet')) {
    if (pm === 'apt') {
      run('wget https://packages.microsoft.com/config/ubuntu/$(lsb_release -rs)/packages-microsoft-prod.deb -O /tmp/msprod.deb');
      run('sudo dpkg -i /tmp/msprod.deb && sudo apt-get update && sudo apt-get install -y dotnet-sdk-8.0');
    } else if (pm === 'dnf') {
      run('sudo dnf install -y dotnet-sdk-8.0');
    } else if (pm === 'pacman') {
      run('sudo pacman -S --noconfirm dotnet-sdk');
    }
  }

  // Tauri CLI
  if (!hasCmd('cargo-tauri')) run('cargo install tauri-cli --version "^2.0"');
}

async function setupMacOS() {
  log('info', 'Обнаружен macOS');
  if (!hasCmd('brew')) throw new Error('Homebrew не найден. Установите с https://brew.sh');

  run('brew install pkg-config');

  if (!hasCmd('cargo')) {
    run('curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y');
    process.env.PATH += ':' + process.env.HOME + '/.cargo/bin';
    run('rustup component add rustfmt clippy');
  }

  if (!hasCmd('node')) run('brew install node@20');

  if (!hasCmd('java')) {
    run('brew install openjdk@17');
    run('sudo ln -sfn $(brew --prefix)/opt/openjdk@17/libexec/openjdk.jdk /Library/Java/JavaVirtualMachines/openjdk-17.jdk');
  }

  if (!hasCmd('dotnet')) run('brew install --cask dotnet-sdk');

  if (!hasCmd('cargo-tauri')) run('cargo install tauri-cli --version "^2.0"');
}

async function setupWindows() {
  log('info', 'Обнаружен Windows');
  if (!hasCmdWin('winget.exe')) throw new Error('winget не найден (Windows 10 1809+ / Windows 11)');

  // Rust
  if (!hasCmdWin('cargo.exe')) {
    run('winget install --id Rustlang.Rust.MSVC --source winget --accept-source-agreements --accept-package-agreements');
    const rustupPath = process.env.USERPROFILE + '\\.cargo\\bin\\rustup.exe';
    run(`"${rustupPath}" component add rustfmt clippy`);
    process.env.PATH += ';' + process.env.USERPROFILE + '\\.cargo\\bin';
  }

  // Node.js
  if (!hasCmdWin('node.exe')) {
    run('winget install --id OpenJS.NodeJS.LTS --source winget --accept-source-agreements --accept-package-agreements');
  }

  // Java 17
  if (!hasCmdWin('java.exe')) {
    run('winget install --id EclipseAdoptium.Temurin.17.JDK --source winget --accept-source-agreements --accept-package-agreements');
  }

  // .NET 8
  if (!hasCmdWin('dotnet.exe')) {
    run('winget install --id Microsoft.DotNet.SDK.8 --source winget --accept-source-agreements --accept-package-agreements');
  }

  // WebView2
  try {
    run('reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"');
  } catch {
    run('winget install --id Microsoft.WebView2Runtime --source winget --accept-source-agreements --accept-package-agreements');
  }

  // VS Build Tools (warning only)
  try {
    run('reg query "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\SxS\\VS7"');
  } catch {
    log('warn', 'Visual Studio Build Tools не найдены. Для нативных модулей установите:');
    log('warn', '  winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget --accept-source-agreements --accept-package-agreements');
    log('warn', 'Выберите: "Desktop development with C++"');
  }

  // Tauri CLI
  if (!hasCmdWin('cargo-tauri.exe')) run('cargo install tauri-cli --version "^2.0"');

  // Git
  if (!hasCmdWin('git.exe')) run('winget install --id Git.Git --source winget --accept-source-agreements --accept-package-agreements');
}

async function main() {
  console.log(`${colors.blue}=== Mini AI 1C Setup ===${colors.reset}`);
  console.log(`OS: ${OS}, ARCH: ${ARCH}`);

  try {
    if (OS === 'linux') await setupLinux();
    else if (OS === 'darwin') await setupMacOS();
    else if (OS === 'win32') await setupWindows();
    else throw new Error(`Неподдерживаемая ОС: ${OS}`);

    console.log(`\n${colors.green}=== Готово! ===${colors.reset}`);
    console.log('Перезапустите терминал для обновления PATH.');
    console.log('Затем:');
    console.log('  cd tauri-app');
    console.log('  npm install');
    console.log('  npm run build:mcp');
  } catch (e) {
    log('err', e.message);
    process.exit(1);
  }
}

main();