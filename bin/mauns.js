#!/usr/bin/env node
"use strict";

const { execFileSync } = require("child_process");
const path = require("path");
const os = require("os");
const fs = require("fs");

const platform = os.platform();
const arch = os.arch();

const BINARY_MAP = {
  "linux-x64":   "mauns-linux-x64",
  "linux-arm64": "mauns-linux-arm64",
  "darwin-x64":  "mauns-macos-x64",
  "darwin-arm64":"mauns-macos-arm64",
  "win32-x64":   "mauns-windows-x64.exe",
};

const key = `${platform}-${arch}`;
const binaryName = BINARY_MAP[key];

if (!binaryName) {
  console.error(`Unsupported platform: ${key}`);
  process.exit(1);
}

const binaryPath = path.join(__dirname, "..", "bin", binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`Mauns binary not found at ${binaryPath}`);
  console.error("Please ensure the package was installed correctly or build from source:");
  console.error("git clone https://github.com/mauns/mauns && cd mauns && cargo build --release");
  process.exit(1);
}

try {
  execFileSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  process.exit(err.status || 1);
}
