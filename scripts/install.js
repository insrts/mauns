"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");
const { execSync } = require("child_process");

const VERSION = require("../package.json").version;
const platform = os.platform();
const arch = os.arch();

const ASSET_MAP = {
  "linux-x64":   `mauns-v${VERSION}-linux-x64.tar.gz`,
  "linux-arm64": `mauns-v${VERSION}-linux-arm64.tar.gz`,
  "darwin-x64":  `mauns-v${VERSION}-macos-x64.tar.gz`,
  "darwin-arm64":`mauns-v${VERSION}-macos-arm64.tar.gz`,
  "win32-x64":   `mauns-v${VERSION}-windows-x64.zip`,
};

const BIN_NAMES = {
  "linux-x64":   "mauns-linux-x64",
  "linux-arm64": "mauns-linux-arm64",
  "darwin-x64":  "mauns-macos-x64",
  "darwin-arm64":"mauns-macos-arm64",
  "win32-x64":   "mauns-windows-x64.exe",
};

const key = `${platform}-${arch}`;
const asset = ASSET_MAP[key];
const binaryName = BIN_NAMES[key];

if (!asset) {
  console.error(`[mauns] Unsupported platform: ${key}. Build from source: https://github.com/mauns/mauns`);
  process.exit(0); // Exit gracefully so npm install doesn't fail
}

const binDir = path.join(__dirname, "..", "bin");
if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

const url = `https://github.com/mauns/mauns/releases/download/v${VERSION}/${asset}`;
const dest = path.join(binDir, asset);

console.log(`[mauns] Downloading ${asset} from ${url}...`);

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        download(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on("finish", () => {
        file.close(resolve);
      });
    }).on("error", (err) => {
      fs.unlink(dest, () => reject(err));
    });
  });
}

async function install() {
  try {
    await download(url, dest);
    console.log(`[mauns] Extracting ${asset}...`);

    if (asset.endsWith(".tar.gz")) {
      execSync(`tar -xzf "${dest}" -C "${binDir}"`);
      const extractedBinary = path.join(binDir, "mauns");
      const targetBinary = path.join(binDir, binaryName);
      if (fs.existsSync(extractedBinary)) {
          fs.renameSync(extractedBinary, targetBinary);
      }
    } else if (asset.endsWith(".zip")) {
      // For Windows, assume powershell or similar is available if zip is used.
      // But actually, we might need a better way if unzip is not there.
      try {
        execSync(`unzip -o "${dest}" -d "${binDir}"`);
      } catch (e) {
        execSync(`powershell -Command "Expand-Archive -Path '${dest}' -DestinationPath '${binDir}' -Force"`);
      }
      const extractedBinary = path.join(binDir, "mauns.exe");
      const targetBinary = path.join(binDir, binaryName);
      if (fs.existsSync(extractedBinary)) {
          fs.renameSync(extractedBinary, targetBinary);
      }
    }

    // Set permissions
    const binaryPath = path.join(binDir, binaryName);
    if (fs.existsSync(binaryPath) && platform !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }

    // Cleanup
    fs.unlinkSync(dest);

    console.log(`[mauns] Install complete.`);
  } catch (err) {
    console.error(`[mauns] Error during installation: ${err.message}`);
    console.error(`[mauns] Please try manual installation or build from source.`);
    process.exit(0); // Exit gracefully
  }
}

install();
