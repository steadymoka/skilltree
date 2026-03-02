#!/usr/bin/env node

"use strict";

const os = require("os");
const path = require("path");
const fs = require("fs");
const https = require("https");
const { execFileSync } = require("child_process");

const REPO = "steadymoka/skill-tree";
const NAME = "skill-tree";

const PLATFORM_MAP = {
  "darwin-arm64": { target: "aarch64-apple-darwin", archive: "tar.gz" },
  "darwin-x64": { target: "x86_64-apple-darwin", archive: "tar.gz" },
  "linux-x64": { target: "x86_64-unknown-linux-gnu", archive: "tar.gz" },
  "linux-arm64": { target: "aarch64-unknown-linux-gnu", archive: "tar.gz" },
  "win32-x64": { target: "x86_64-pc-windows-msvc", archive: "zip" },
};

function getPlatformInfo() {
  const key = `${os.platform()}-${os.arch()}`;
  const info = PLATFORM_MAP[key];
  if (!info) {
    throw new Error(
      `Unsupported platform: ${key}\nSupported: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
  }
  return info;
}

function getVersion() {
  const pkg = JSON.parse(
    fs.readFileSync(path.join(__dirname, "package.json"), "utf8")
  );
  return pkg.version;
}

function fetch(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "skill-tree-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return fetch(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

function extractTarGz(buffer, destDir) {
  const archivePath = path.join(destDir, "archive.tar.gz");
  fs.writeFileSync(archivePath, buffer);
  execFileSync("tar", ["xzf", archivePath, "-C", destDir]);
  fs.unlinkSync(archivePath);
}

function extractZip(buffer, destDir) {
  const archivePath = path.join(destDir, "archive.zip");
  fs.writeFileSync(archivePath, buffer);
  execFileSync("powershell", [
    "-NoProfile",
    "-Command",
    `Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force`,
  ]);
  fs.unlinkSync(archivePath);
}

async function main() {
  const { target, archive } = getPlatformInfo();
  const version = getVersion();
  const binDir = path.join(__dirname, "bin");
  const binaryName = os.platform() === "win32" ? `${NAME}.exe` : NAME;
  const binaryPath = path.join(binDir, binaryName);

  if (fs.existsSync(binaryPath)) {
    console.log(`${NAME} binary already exists at ${binaryPath}`);
    return;
  }

  const url = `https://github.com/${REPO}/releases/download/v${version}/${NAME}-${target}.${archive}`;
  console.log(`Downloading ${NAME} v${version} for ${target}...`);

  const buffer = await fetch(url);

  fs.mkdirSync(binDir, { recursive: true });

  if (archive === "tar.gz") {
    extractTarGz(buffer, binDir);
  } else {
    extractZip(buffer, binDir);
  }

  if (os.platform() !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }

  console.log(`Installed ${NAME} to ${binaryPath}`);
}

main().catch((err) => {
  console.error(`Failed to install ${NAME}: ${err.message}`);
  process.exit(1);
});
