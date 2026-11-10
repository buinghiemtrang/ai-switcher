#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const { execFileSync, spawn } = require('node:child_process');

const REPOSITORY = 'buinghiemtrang/ai-switcher';
const RELEASE = 'v0.12.2-linux.1';
const MANIFEST = '.ai-switcher-install.json';

function assertPlatform(platform = process.platform, architecture = process.arch) {
  if (platform !== 'linux' || architecture !== 'x64') {
    throw new Error('AI switcher hiện hỗ trợ Linux x86_64 (Ubuntu 24.04).');
  }
}

function digest(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function verifyChecksum(file, checksumText) {
  const name = path.basename(file);
  const entries = checksumText.split(/\r?\n/).filter(Boolean).map((line) => {
    const match = /^([a-fA-F0-9]{64})[ \t]+\*?(.+)$/.exec(line);
    if (!match) throw new Error('SHA256SUMS không hợp lệ.');
    return { hash: match[1].toLowerCase(), name: match[2] };
  });
  const matches = entries.filter((entry) => entry.name === name);
  if (matches.length !== 1 || matches[0].hash !== digest(file)) {
    throw new Error('Checksum của gói cài không khớp; chưa thay đổi ứng dụng hiện tại.');
  }
  return matches[0].hash;
}

function desktopQuote(value) {
  if (/[\r\n\0]/.test(value)) throw new Error('Đường dẫn cài đặt không hợp lệ.');
  return `"${value.replace(/%/g, '%%').replace(/[\\"$`]/g, '\\$&')}"`;
}

function locations(home = os.homedir(), env = process.env) {
  const root = path.join(home, '.local', 'opt', 'ai-switcher');
  const dataHome = env.XDG_DATA_HOME && path.isAbsolute(env.XDG_DATA_HOME)
    ? env.XDG_DATA_HOME : path.join(home, '.local', 'share');
  return {
    root,
    binary: path.join(root, 'usr', 'bin', 'ai-account-switcher'),
    launcher: path.join(home, '.local', 'bin', 'ai-switcher'),
    desktop: path.join(dataHome, 'applications', 'ai-switcher.desktop'),
  };
}

function desktopContents(paths) {
  const icon = path.join(paths.root, 'usr', 'share', 'icons', 'hicolor', '128x128', 'apps', 'ai-account-switcher.png');
  return '# Managed by AI switcher npx installer\n[Desktop Entry]\nName=AI switcher\n'
    + `Exec=${desktopQuote(paths.binary)}\nIcon=${icon}\nTerminal=false\nType=Application\n`
    + 'Categories=Development;Utility;\nStartupWMClass=ai-account-switcher\n';
}

function assertManagedPaths(paths) {
  if (fs.existsSync(paths.root)) {
    const stat = fs.lstatSync(paths.root);
    const oldDesktop = path.join(paths.root, 'usr', 'share', 'applications', 'AI switcher.desktop');
    const oldPackage = fs.existsSync(oldDesktop) && fs.readFileSync(oldDesktop, 'utf8').includes('Name=AI switcher\n');
    const owned = fs.existsSync(path.join(paths.root, MANIFEST)) || oldPackage;
    if (stat.isSymbolicLink() || !stat.isDirectory() || !owned || !fs.existsSync(paths.binary)) {
      throw new Error(`Không ghi đè thư mục không được nhận diện là AI switcher: ${paths.root}`);
    }
  }
  try {
    const stat = fs.lstatSync(paths.launcher);
    const sameLink = stat.isSymbolicLink()
      && path.resolve(path.dirname(paths.launcher), fs.readlinkSync(paths.launcher)) === paths.binary;
    if (!sameLink) throw new Error(`Không ghi đè launcher đã có: ${paths.launcher}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  let desktopStat;
  try { desktopStat = fs.lstatSync(paths.desktop); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (desktopStat) {
    if (desktopStat.isSymbolicLink() || !desktopStat.isFile()) throw new Error(`Không ghi đè desktop symlink hoặc thư mục: ${paths.desktop}`);
    const text = fs.readFileSync(paths.desktop, 'utf8');
    const owned = text.startsWith('# Managed by AI switcher npx installer\n')
      || (text.includes('Name=AI switcher\n') && text.includes(`Exec=${paths.binary}\n`));
    if (!owned) throw new Error(`Không ghi đè mục ứng dụng đã có: ${paths.desktop}`);
  }
}

function registerLaunchers(paths) {
  fs.mkdirSync(path.dirname(paths.launcher), { recursive: true });
  fs.mkdirSync(path.dirname(paths.desktop), { recursive: true });
  try {
    fs.symlinkSync(paths.binary, paths.launcher);
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
  }
  fs.writeFileSync(paths.desktop, desktopContents(paths), { mode: 0o644 });
}

function commandExists(command) {
  try {
    execFileSync(command, ['--version'], { stdio: 'ignore', timeout: 10000 });
    return true;
  } catch { return false; }
}

function checkRuntime(binary) {
  let output;
  try {
    output = execFileSync('ldd', [binary], { encoding: 'utf8', timeout: 10000, stdio: ['ignore', 'pipe', 'pipe'] });
  } catch {
    throw new Error('Không kiểm tra được thư viện runtime. Hãy dùng cách cài .deb trong README.');
  }
  const missing = output.split('\n').filter((line) => line.includes('not found')).map((line) => line.trim().split(/\s/)[0]);
  if (missing.length) {
    throw new Error(`Thiếu thư viện Linux: ${missing.join(', ')}. Hãy cài file .deb bằng sudo apt install để bổ sung runtime.`);
  }
}

function cachedInstall(paths) {
  try {
    const manifest = JSON.parse(fs.readFileSync(path.join(paths.root, MANIFEST), 'utf8'));
    return manifest.repository === REPOSITORY && manifest.release === RELEASE
      && manifest.binarySha256 === digest(paths.binary);
  } catch { return false; }
}

function install(paths, update = false) {
  assertManagedPaths(paths);
  if (!update && cachedInstall(paths)) {
    checkRuntime(paths.binary);
    registerLaunchers(paths);
    console.log(`AI switcher ${RELEASE} đã được cài.`);
    return;
  }
  for (const command of ['gh', 'dpkg-deb']) {
    if (!commandExists(command)) {
      throw new Error(`Cần lệnh ${command}. Xem phần cài bằng npx hoặc .deb trong README.`);
    }
  }
  const parent = path.dirname(paths.root);
  fs.mkdirSync(parent, { recursive: true });
  const lock = path.join(parent, '.ai-switcher-install.lock');
  try { fs.mkdirSync(lock, { mode: 0o700 }); }
  catch (error) {
    if (error.code === 'EEXIST') throw new Error('Đang có phiên cài AI switcher khác, hoặc lần cài trước bị ngắt. Kiểm tra trước khi xóa ~/.local/opt/.ai-switcher-install.lock.');
    throw error;
  }
  const work = fs.mkdtempSync(path.join(parent, '.ai-switcher-install-'));
  fs.chmodSync(work, 0o700);
  const downloaded = path.join(work, 'download');
  const staged = path.join(work, 'app');
  const backup = path.join(work, 'previous');
  fs.mkdirSync(downloaded);
  let replaced = false;
  let committed = false;
  try {
    console.log(`Đang tải AI switcher ${RELEASE}…`);
    try {
      execFileSync('gh', ['release', 'download', RELEASE, '--repo', REPOSITORY, '--pattern', '*.deb', '--pattern', 'SHA256SUMS', '--dir', downloaded], {
        stdio: ['ignore', 'ignore', 'pipe'], timeout: 180000,
      });
    } catch {
      throw new Error('Không tải được bản phát hành. Chạy gh auth login và kiểm tra quyền truy cập repo buinghiemtrang/ai-switcher.');
    }
    const packages = fs.readdirSync(downloaded).filter((name) => name.endsWith('.deb'));
    if (packages.length !== 1) throw new Error('Bản phát hành phải có đúng một gói .deb cho Linux x86_64.');
    const deb = path.join(downloaded, packages[0]);
    const packageSha256 = verifyChecksum(deb, fs.readFileSync(path.join(downloaded, 'SHA256SUMS'), 'utf8'));
    execFileSync('dpkg-deb', ['--extract', deb, staged], { stdio: ['ignore', 'ignore', 'pipe'], timeout: 60000 });
    const binary = path.join(staged, 'usr', 'bin', 'ai-account-switcher');
    fs.accessSync(binary, fs.constants.X_OK);
    checkRuntime(binary);
    fs.writeFileSync(path.join(staged, MANIFEST), JSON.stringify({
      repository: REPOSITORY, release: RELEASE, packageSha256,
      binarySha256: digest(binary),
    }, null, 2) + '\n', { mode: 0o600 });
    if (fs.existsSync(paths.root)) fs.renameSync(paths.root, backup);
    try {
      fs.renameSync(staged, paths.root);
      replaced = true;
      registerLaunchers(paths);
      committed = true;
    } catch (error) {
      if (replaced) fs.renameSync(paths.root, staged);
      if (fs.existsSync(backup)) fs.renameSync(backup, paths.root);
      throw error;
    }
    console.log('Đã cài AI switcher. Mở từ menu ứng dụng hoặc chạy ~/.local/bin/ai-switcher.');
  } finally {
    // Never erase the old installation if an unexpected filesystem error prevented rollback.
    if (committed || !fs.existsSync(backup)) fs.rmSync(work, { recursive: true, force: true });
    else console.error(`Bản cài cũ được giữ để khôi phục tại: ${backup}`);
    fs.rmdirSync(lock);
  }
}

async function main(args = process.argv.slice(2)) {
  if (args.includes('--help') || args.includes('-h')) {
    console.log('AI switcher — Linux x86_64\n\nCách dùng: ai-switcher [--install-only] [--update]\n  --install-only  Cài ứng dụng, không mở cửa sổ\n  --update        Tải và cài lại bản phát hành được ghim\n\nCần Node.js 20+, GitHub CLI (gh), quyền truy cập repo và các thư viện runtime Linux.\nKhông cần Rust hoặc build source.');
    return;
  }
  if (args.some((arg) => !['--install-only', '--update'].includes(arg))) throw new Error('Tham số không hợp lệ. Chạy ai-switcher --help.');
  assertPlatform();
  const paths = locations();
  install(paths, args.includes('--update'));
  if (!args.includes('--install-only')) {
    if (!process.env.DISPLAY && !process.env.WAYLAND_DISPLAY) {
      throw new Error('Đã cài xong. Cần phiên desktop để mở AI switcher; hoặc dùng --install-only.');
    }
    await new Promise((resolve, reject) => {
      const child = spawn(paths.binary, [], { detached: true, stdio: 'ignore' });
      child.once('error', reject);
      child.once('spawn', () => { child.unref(); resolve(); });
    });
    console.log('Đã mở AI switcher.');
  }
}

module.exports = { assertPlatform, verifyChecksum, desktopQuote, desktopContents, locations, assertManagedPaths, cachedInstall, install, main };
if (require.main === module) main().catch((error) => { console.error(`AI switcher: ${error.message}`); process.exitCode = 1; });
