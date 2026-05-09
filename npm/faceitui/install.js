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
  console.error(`Unsupported platform: ${platform}`);
  console.error('Supported: win32-x64, linux-x64, darwin-x64, darwin-arm64');
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

console.log(`Downloading faceitui v${VERSION} for ${platform}...`);
console.log(downloadUrl);

if (!fs.existsSync(BIN_DIR)) {
  fs.mkdirSync(BIN_DIR, { recursive: true });
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
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
        reject(new Error(`HTTP ${response.statusCode}: ${url}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        resolve();
      });
    }).on('error', (err) => {
      file.close();
      fs.unlinkSync(dest);
      reject(err);
    });
  });
}

async function main() {
  const tempArchive = path.join(BIN_DIR, `temp${downloadExt}`);

  try {
    await download(downloadUrl, tempArchive);
  } catch (err) {
    console.error(`Download failed: ${err.message}`);
    console.error('Check that the release exists: ' + `${repoUrl}/releases/tag/${tag}`);
    process.exit(1);
  }

  if (isWindows) {
    try {
      execSync(`powershell -Command "Expand-Archive -Path '${tempArchive}' -DestinationPath '${BIN_DIR}' -Force"`, { stdio: 'inherit' });
    } catch {
      // fallback: try tar on windows
      try {
        execSync(`tar -xf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'inherit' });
      } catch {
        console.error('Failed to extract. Install 7-Zip or enable tar on Windows.');
        process.exit(1);
      }
    }
  } else {
    execSync(`tar -xzf "${tempArchive}" -C "${BIN_DIR}"`, { stdio: 'inherit' });
  }

  const binaryPath = path.join(BIN_DIR, binaryName);
  if (!fs.existsSync(binaryPath)) {
    console.error('Binary not found after extraction.');
    process.exit(1);
  }

  if (!isWindows) {
    fs.chmodSync(binaryPath, 0o755);
  }

  fs.unlinkSync(tempArchive);
  console.log(`faceitui v${VERSION} installed successfully!`);
}

main();
