const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const BIN_DIR = path.join(__dirname, 'bin');
const PACKAGE_JSON = require('./package.json');
const VERSION = PACKAGE_JSON.version;

const PLATFORM_MAP = {
  'win32-x64':   'x86_64-pc-windows-msvc',
  'linux-x64':   'x86_64-unknown-linux-gnu',
  'darwin-x64':  'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
};

const platform = `${process.platform}-${process.arch}`;
const target = PLATFORM_MAP[platform];

if (!target) {
  log('x', 'red', `Unsupported platform: ${platform}`);
  log('i', 'dim', 'Supported: win32-x64, linux-x64, darwin-x64, darwin-arm64');
  process.exit(1);
}

const isWindows = process.platform === 'win32';
const ext = isWindows ? 'zip' : 'tar.gz';
const downloadExt = isWindows ? '.zip' : '.tar.gz';
const binaryName = `faceitui${isWindows ? '.exe' : ''}`;

const repoUrl = 'https://github.com/cnaltn/Faceitui';
const tag = `v${VERSION}`;
const archiveName = `faceitui-${target}.${ext}`;
const downloadUrl = `${repoUrl}/releases/download/${tag}/${archiveName}`;

const C = {
  reset:   '\x1b[0m',
  bold:    '\x1b[1m',
  dim:     '\x1b[2m',
  cyan:    '\x1b[36m',
  green:   '\x1b[32m',
  yellow:  '\x1b[33m',
  red:     '\x1b[31m',
  white:   '\x1b[97m',
};

function color(code, text) {
  return code + text + C.reset;
}

const SPINNER = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

function log(icon, clr, msg) {
  const iconColors = {
    green:  C.green,
    red:    C.red,
    cyan:   C.cyan,
    yellow: C.yellow,
    dim:    C.dim,
  };
  const clrCode = iconColors[clr] || C.white;
  process.stdout.write(`  ${clrCode}${icon}${C.reset} ${msg}\n`);
}

function progressBar(current, total, width) {
  const pct = Math.min(current / total, 1);
  const filled = Math.round(pct * width);
  const empty = width - filled;
  return `${C.cyan}${'█'.repeat(filled)}${C.dim}${'░'.repeat(empty)}${C.reset}`;
}

function fmtSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

async function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    let downloaded = 0;
    let totalSize = 0;
    let spinnerIdx = 0;
    let spinnerTimer = null;
    let lastLogLine = '';

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        file.close();
        fs.unlinkSync(dest);
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        file.close();
        fs.unlinkSync(dest);
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }

      totalSize = parseInt(response.headers['content-length'], 10) || 0;
      const displaySize = totalSize > 0 ? fmtSize(totalSize) : '?';

      spinnerTimer = setInterval(() => {
        const spinner = color(C.cyan, SPINNER[spinnerIdx]);
        const bar = totalSize > 0
          ? `[${progressBar(downloaded, totalSize, 20)}] ${Math.round((downloaded / totalSize) * 100)}%`
          : ` ${fmtSize(downloaded)} / ${displaySize}`;
        const line = `  ${spinner} Downloading...  ${bar}`;
        process.stdout.write(`\r${line}`);
        lastLogLine = line;
        spinnerIdx = (spinnerIdx + 1) % SPINNER.length;
      }, 80);

      response.on('data', (chunk) => {
        downloaded += chunk.length;
      });

      response.pipe(file);

      file.on('finish', () => {
        clearInterval(spinnerTimer);
        file.close();
        const bar = totalSize > 0
          ? `[${progressBar(totalSize, totalSize, 20)}] 100%`
          : ` ${fmtSize(downloaded)}`;
        process.stdout.write(`\r${' '.repeat(lastLogLine.length)}\r`);
        log('✓', 'green', `Downloaded (${fmtSize(downloaded)})  ${bar}`);
        resolve();
      });
    }).on('error', (err) => {
      if (spinnerTimer) clearInterval(spinnerTimer);
      file.close();
      try { fs.unlinkSync(dest); } catch {}
      reject(err);
    });
  });
}

function banner() {
  const lines = [
    '    ███████╗ █████╗  ██████╗███████╗██╗████████╗██╗   ██╗██╗',
    '    ██╔════╝██╔══██╗██╔════╝██╔════╝██║╚══██╔══╝██║   ██║██║',
    '    █████╗  ███████║██║     █████╗  ██║   ██║   ██║   ██║██║',
    '    ██╔══╝  ██╔══██║██║     ██╔══╝  ██║   ██║   ██║   ██║██║',
    '    ██║     ██║  ██║╚██████╗███████╗██║   ██║   ╚██████╔╝██║',
    '    ╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝╚═╝   ╚═╝    ╚═════╝ ╚═╝',
  ];
  process.stdout.write('\n');
  for (const line of lines) {
    let colored = '';
    for (const ch of line) {
      if (ch === ' ' || ch === '\n') {
        colored += ch;
      } else {
        colored += color(C.cyan, ch);
      }
    }
    process.stdout.write(colored + '\n');
  }
  process.stdout.write('\n');
  process.stdout.write(color(C.bold, `     FACEIT CS2 Stats TUI  —  ${color(C.dim, 'v' + VERSION)}`));
  process.stdout.write('\n\n');
}

function platformLine() {
  const osNames = { win32: 'Windows', darwin: 'macOS', linux: 'Linux' };
  const osName = osNames[process.platform] || process.platform;
  const archName = process.arch === 'x64' ? 'x86-64' : process.arch;
  process.stdout.write(`  ${color(C.yellow, '◉')} ${color(C.white, 'Platform')}  ${osName} (${archName})  ${color(C.dim, '\u2192')}  ${color(C.dim, target)}\n\n`);
}

async function main() {
  banner();
  platformLine();

  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }

  const tempArchive = path.join(BIN_DIR, `temp${downloadExt}`);

  try {
    await download(downloadUrl, tempArchive);
  } catch (err) {
    log('✗', 'red', `Download failed: ${err.message}`);
    log('i', 'dim', `Release URL: ${repoUrl}/releases/tag/${tag}`);
    process.exit(1);
  }

  log('✓', 'green', 'Extracting...');

  if (isWindows) {
    try {
      execSync(`powershell -Command "Expand-Archive -Path '${tempArchive}' -DestinationPath '${BIN_DIR}' -Force"`, { stdio: 'pipe' });
    } catch {
      try {
        execSync(`tar -xf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'pipe' });
      } catch {
        log('✗', 'red', 'Failed to extract. Install 7-Zip or enable tar on Windows.');
        process.exit(1);
      }
    }
  } else {
    execSync(`tar -xzf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'pipe' });
  }

  const binaryPath = path.join(BIN_DIR, binaryName);
  if (!fs.existsSync(binaryPath)) {
    log('✗', 'red', 'Binary not found after extraction.');
    process.exit(1);
  }

  if (!isWindows) {
    fs.chmodSync(binaryPath, 0o755);
  }

  fs.unlinkSync(tempArchive);

  log('✓', 'green', 'Installed!');
  process.stdout.write(`\n  ${color(C.cyan, '\u25b8')} run: ${color(C.bold, 'faceitui')}\n\n`);
}

main();
