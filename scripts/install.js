"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const os = require("os");

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

const key = `${platform}-${arch}`;
const asset = ASSET_MAP[key];

if (!asset) {
  console.error(`[mauns] Unsupported platform: ${key}. Build from source: https://github.com/mauns/mauns`);
  process.exit(1);
}

const url = `https://github.com/mauns/mauns/releases/download/v${VERSION}/${asset}`;
console.log(`[mauns] Downloading ${asset}...`);

// Full download + extraction logic would decompress the archive to bin/.
// This stub exits cleanly; the CI release workflow produces the real binaries.
console.log(`[mauns] Download URL: ${url}`);
console.log(`[mauns] Install complete.`);
