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
  fail(`Unsupported platform: ${platform}`);
  info('Supported: win32-x64, linux-x64, darwin-x64, darwin-arm64');
  process.exit(1);
}

const isWindows = process.platform === 'win32';

if (isWindows) {
  try {
    execSync('chcp 65001', { stdio: 'pipe' });
  } catch {}
}

const ext = isWindows ? 'zip' : 'tar.gz';
const downloadExt = isWindows ? '.zip' : '.tar.gz';
const binaryName = `faceitui${isWindows ? '.exe' : ''}`;

const repoUrl = 'https://github.com/cnaltn/Faceitui';
const tag = `v${VERSION}`;
const archiveName = `faceitui-${target}.${ext}`;
const downloadUrl = `${repoUrl}/releases/download/${tag}/${archiveName}`;

function ewrite(s) {
  fs.writeSync(2, s);
}

function color(code, text) {
  return `\x1b[${code}m${text}\x1b[0m`;
}

function ok(msg)      { ewrite(`  ${color('32', 'ok')} ${msg}\n`); }
function fail(msg)    { ewrite(`  ${color('31', '!!')} ${msg}\n`); }
function info(msg)    { ewrite(`     ${color('2', msg)}\n`); }
function dim(msg)     { return color('2', msg); }

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
    let spinnerTimer = null;
    const start = Date.now();

    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        file.close();
        try { fs.unlinkSync(dest); } catch {}
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        file.close();
        try { fs.unlinkSync(dest); } catch {}
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }

      totalSize = parseInt(response.headers['content-length'], 10) || 0;

      spinnerTimer = setInterval(() => {
        let bar = '';
        if (totalSize > 0) {
          const pct = Math.min(downloaded / totalSize, 1);
          const w = 24;
          const filled = Math.round(pct * w);
          bar = `[${'='.repeat(filled)}${' '.repeat(w - filled)}] ${Math.round(pct * 100)}%`;
        } else {
          bar = `${fmtSize(downloaded)} received`;
        }
        ewrite(`\r  ${color(36, 'downloading')}  ${bar}`);
      }, 100);

      response.on('data', (chunk) => {
        downloaded += chunk.length;
      });

      response.pipe(file);

      file.on('finish', () => {
        clearInterval(spinnerTimer);
        file.close();
        const elapsed = ((Date.now() - start) / 1000).toFixed(1);
        ewrite(`\r${' '.repeat(60)}\r`);
        ok(`Downloaded ${fmtSize(downloaded)} in ${elapsed}s`);
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
  const ACCENT = '\x1b[38;2;218;112;44m'; // flexoki-dark orange #da702c
  const RST = '\x1b[0m';
  const lines = [
    '    ███████╗ █████╗  ██████╗███████╗██╗████████╗██╗   ██╗██╗',
    '    ██╔════╝██╔══██╗██╔════╝██╔════╝██║╚══██╔══╝██║   ██║██║',
    '    █████╗  ███████║██║     █████╗  ██║   ██║   ██║   ██║██║',
    '    ██╔══╝  ██╔══██║██║     ██╔══╝  ██║   ██║   ██║   ██║██║',
    '    ██║     ██║  ██║╚██████╗███████╗██║   ██║   ╚██████╔╝██║',
    '    ╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝╚═╝   ╚═╝    ╚═════╝ ╚═╝',
  ];
  ewrite('\n');
  for (const line of lines) {
    let out = '';
    for (const ch of line) {
      out += (ch === ' ') ? ' ' : `${ACCENT}${ch}${RST}`;
    }
    ewrite(out + '\n');
  }
  ewrite('\n');
  ewrite(`${ACCENT}     FACEIT CS2 Stats TUI  -  \x1b[2mv${VERSION}${RST}\n\n`);
}

function platformLine() {
  const osNames = { win32: 'Windows', darwin: 'macOS', linux: 'Linux' };
  const osName = osNames[process.platform] || process.platform;
  const archName = process.arch === 'x64' ? 'x86-64' : process.arch;
  ewrite(`  ${color('33', '*')} ${color('37', 'Platform')}  ${osName} (${archName})  ${dim('->')}  ${dim(target)}\n\n`);
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
    ewrite('\n');
    fail(`Download failed: ${err.message}`);
    info(`Release URL: ${repoUrl}/releases/tag/${tag}`);
    process.exit(1);
  }

  ok('Extracting...');

  if (isWindows) {
    try {
      execSync(`powershell -Command "Expand-Archive -Path '${tempArchive}' -DestinationPath '${BIN_DIR}' -Force"`, { stdio: 'pipe' });
    } catch {
      try {
        execSync(`tar -xf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'pipe' });
      } catch {
        fail('Failed to extract. Install 7-Zip or enable tar on Windows.');
        process.exit(1);
      }
    }
  } else {
    execSync(`tar -xzf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'pipe' });
  }

  const binaryPath = path.join(BIN_DIR, binaryName);
  if (!fs.existsSync(binaryPath)) {
    fail('Binary not found after extraction.');
    process.exit(1);
  }

  if (!isWindows) {
    fs.chmodSync(binaryPath, 0o755);
  }

  fs.unlinkSync(tempArchive);

  ok('Installed!');
  ewrite(`\n  ${color('36', '>')} run: \x1b[1mfaceitui\x1b[0m\n\n`);

  try {
    execSync('npm config set foreground-scripts true', { stdio: 'pipe' });
  } catch {}

  process.exit(0);
}

main().catch((err) => {
  fail(`Unexpected error: ${err.message}`);
  process.exit(1);
});
