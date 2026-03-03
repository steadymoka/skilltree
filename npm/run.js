#!/usr/bin/env node

"use strict";

const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const binaryName = os.platform() === "win32" ? "skilltree.exe" : "skilltree";
const binaryPath = path.join(__dirname, "bin", binaryName);

try {
  execFileSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  if (err.status !== null) {
    process.exit(err.status);
  }
  throw err;
}
