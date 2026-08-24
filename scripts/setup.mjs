#!/usr/bin/env node
// Mini AI 1C — Universal setup script (Node.js)
// Usage: node scripts/setup.mjs
// Or add to package.json: "setup": "node scripts/setup.mjs"

import { execSync } from 'node:child_process';
import { platform, arch } from 'node:os';
import { existsSync, readdirSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';

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
  // First check PATH via where
  try {
    execSync(`where ${cmd}`, { stdio: 'ignore', windowsHide: true });
    return true;
  } catch {}
  return false;
}

// Возвращает команду для запуска: имя (если в PATH) или полный путь в кавычках, либо null
function resolveWindowsTool(name, extraPaths = []) {
  try {
    execSync(`where ${name}`, { stdio: 'ignore', windowsHide: true });
    return name;
  } catch {}
  for (const p of extraPaths) {
    if (p && existsSync(p)) return `"${p}"`;
  }
  return null;
}

function run(cmd, opts = {}) {
  log('info', `$ ${cmd}`);
  execSync(cmd, { stdio: 'inherit', ...opts });
}

// Код 0x8A15002B = "обновление неприменимо / уже установлена последняя версия"
const WINGET_ALREADY_INSTALLED = -1978335189;

function runWingetInstall(id) {
  log('info', `$ winget install --id ${id}`);
  let out = '';
  try {
    out = execSync(
      `winget install --id ${id} --source winget --accept-source-agreements --accept-package-agreements`,
      { encoding: 'utf8', windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] },
    ) || '';
  } catch (e) {
    out = String(e.stdout || '') + String(e.stderr || '');
    if (e.status === WINGET_ALREADY_INSTALLED || /no available upgrade|already installed/i.test(out)) {
      log('info', `Пакет ${id} уже установлен, пропускаем`);
      console.log(out.trim());
      return;
    }
    if (out.trim()) console.log(out.trim());
    throw e;
  }
  if (out.trim()) console.log(out.trim());
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

    // --- Rust ---
  // Ищем cargo: PATH → %USERPROFILE%\.cargo\bin → Program Files (MSI-установка)
  const cargoHome = join(process.env.USERPROFILE || '', '.cargo');
  const findMsiCargo = () => {
    try {
      for (const d of readdirSync('C:\\Program Files')) {
        if (/^Rust/i.test(d)) {
          const p = join('C:\\Program Files', d, 'bin', 'cargo.exe');
          if (existsSync(p)) return p;
        }
      }
    } catch {}
    return null;
  };

  let cargo = resolveWindowsTool('cargo.exe', [join(cargoHome, 'bin', 'cargo.exe'), findMsiCargo()]);

  if (!cargo) {
    // Устанавливаем через rustup: предсказуемые пути, rustup доступен для компонентов
    log('info', 'Установка Rust через rustup...');
    const rustupInit = join(process.env.TEMP || '.', 'rustup-init.exe');
    run(`curl.exe -fsSL -o "${rustupInit}" https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe`);
    run(`"${rustupInit}" -y --default-toolchain stable-x86_64-pc-windows-msvc`);
    try { unlinkSync(rustupInit); } catch {}
    process.env.PATH += ';' + join(cargoHome, 'bin');
    run(`"${join(cargoHome, 'bin', 'rustup.exe')}" component add rustfmt clippy`);
    cargo = `"${join(cargoHome, 'bin', 'cargo.exe')}"`;
    log('info', 'Rust установлен: ' + execSync(`${cargo} --version`, { encoding: 'utf8' }).trim());
  } else {
    log('info', 'Rust найден: ' + execSync(`${cargo} --version`, { encoding: 'utf8' }).trim());
    const rustup = resolveWindowsTool('rustup.exe', [join(cargoHome, 'bin', 'rustup.exe')]);
    if (rustup) {
      try { run(`${rustup} component add rustfmt clippy`); } catch {}
    }
  }

  // Node.js
  if (!hasCmdWin('node.exe')) {
    runWingetInstall('OpenJS.NodeJS.LTS');
  }

  // Java 17
  if (!hasCmdWin('java.exe')) {
    runWingetInstall('EclipseAdoptium.Temurin.17.JDK');
    // Добавляем в PATH текущего процесса свежеустановленный JDK
    const adoptium = 'C:\\Program Files\\Eclipse Adoptium';
    try {
      for (const d of readdirSync(adoptium)) {
        if (/^jdk-17/i.test(d)) {
          process.env.PATH += ';' + join(adoptium, d, 'bin');
          break;
        }
      }
    } catch {}
  }

  // .NET 8
  if (!hasCmdWin('dotnet.exe')) {
    runWingetInstall('Microsoft.DotNet.SDK.8');
  }

  // WebView2
  try {
    run('reg query "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"');
  } catch {
    runWingetInstall('Microsoft.WebView2Runtime');
  }

  // VS Build Tools (warning only)
  try {
    run('reg query "HKLM\\SOFTWARE\\Microsoft\\VisualStudio\\SxS\\VS7"');
  } catch {
    log('warn', 'Visual Studio Build Tools не найдены. Для нативных модулей установите:');
    log('warn', '  winget install --id Microsoft.VisualStudio.2022.BuildTools --source winget --accept-source-agreements --accept-package-agreements');
    log('warn', 'Выберите: "Desktop development with C++"');
  }

  // Tauri CLI (используем найденный cargo — он может быть вне PATH текущего процесса)
  if (!hasCmdWin('cargo-tauri.exe')) run(`${cargo} install tauri-cli --version "^2.0"`);

  // Git
  if (!hasCmdWin('git.exe')) runWingetInstall('Git.Git');
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