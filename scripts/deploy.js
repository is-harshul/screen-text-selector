#!/usr/bin/env node
// Release automation for Text Extractor.
//
// Usage:
//   node scripts/deploy.js <patch|minor|major>
//   npm run deploy -- patch
//
// What it does:
//   1. Reads the current version from package.json.
//   2. Bumps it (semver) per the requested level.
//   3. Rewrites the version in package.json, src-tauri/Cargo.toml,
//      src-tauri/tauri.conf.json, and every occurrence in README.md.
//   4. Commits, tags `vX.Y.Z`, and pushes main + the tag to origin.
//
// The pushed tag triggers .github/workflows/release.yml, which builds the
// installers for every platform and publishes the GitHub Release.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const LEVELS = ["patch", "minor", "major"];

function fail(msg) {
  console.error(`\x1b[31m✗ ${msg}\x1b[0m`);
  process.exit(1);
}

function step(msg) {
  console.log(`\x1b[36m==>\x1b[0m ${msg}`);
}

// Run git, inheriting stdio for pushes so the user sees progress.
function git(args, { capture = false } = {}) {
  return execFileSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
}

function bump(version, level) {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version.trim());
  if (!m) fail(`current version "${version}" is not plain semver (x.y.z)`);
  let [major, minor, patch] = m.slice(1).map(Number);
  if (level === "major") { major++; minor = 0; patch = 0; }
  else if (level === "minor") { minor++; patch = 0; }
  else { patch++; }
  return `${major}.${minor}.${patch}`;
}

// Replace `field: "x.y.z"`-style version in JSON without reformatting the file.
function replaceJsonVersion(path, oldV, newV) {
  const p = resolve(ROOT, path);
  const text = readFileSync(p, "utf8");
  const re = new RegExp(`("version"\\s*:\\s*")${escapeRe(oldV)}(")`);
  if (!re.test(text)) fail(`${path}: no "version": "${oldV}" found`);
  writeFileSync(p, text.replace(re, `$1${newV}$2`));
}

// Replace the first `version = "x.y.z"` (the [package] version) in Cargo.toml.
function replaceCargoVersion(path, oldV, newV) {
  const p = resolve(ROOT, path);
  const text = readFileSync(p, "utf8");
  const re = new RegExp(`(^version\\s*=\\s*")${escapeRe(oldV)}(")`, "m");
  if (!re.test(text)) fail(`${path}: no version = "${oldV}" found`);
  writeFileSync(p, text.replace(re, `$1${newV}$2`));
}

// Replace every occurrence of the old version string in README.md.
function replaceAllInReadme(path, oldV, newV) {
  const p = resolve(ROOT, path);
  const text = readFileSync(p, "utf8");
  const re = new RegExp(escapeRe(oldV), "g");
  const count = (text.match(re) || []).length;
  writeFileSync(p, text.replace(re, newV));
  return count;
}

function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// ---- main ------------------------------------------------------------------

const level = process.argv[2];
if (!LEVELS.includes(level)) {
  fail(`usage: node scripts/deploy.js <${LEVELS.join("|")}>`);
}

// Preflight: clean tree, on main, origin reachable.
const status = git(["status", "--porcelain"], { capture: true }).trim();
if (status) fail("working tree is dirty — commit or stash first:\n" + status);

const branch = git(["rev-parse", "--abbrev-ref", "HEAD"], { capture: true }).trim();
if (branch !== "main") fail(`not on main (on "${branch}") — release from main only`);

const pkgPath = resolve(ROOT, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const oldV = pkg.version;
const newV = bump(oldV, level);
const tag = `v${newV}`;

// Refuse to reuse an existing tag.
const tags = git(["tag"], { capture: true }).split("\n").map((t) => t.trim());
if (tags.includes(tag)) fail(`tag ${tag} already exists`);

step(`Releasing ${oldV} → ${newV} (${level})`);

replaceJsonVersion("package.json", oldV, newV);
replaceCargoVersion("src-tauri/Cargo.toml", oldV, newV);
replaceJsonVersion("src-tauri/tauri.conf.json", oldV, newV);
const readmeHits = replaceAllInReadme("README.md", oldV, newV);
step(`Rewrote version in package.json, Cargo.toml, tauri.conf.json, README.md (${readmeHits} refs)`);

// Keep Cargo.lock's own package entry in sync so the build isn't dirtied.
try {
  git(["add", "-A"]);
  execFileSync("cargo", ["update", "-p", "text-extractor", "--precise", newV], {
    cwd: resolve(ROOT, "src-tauri"),
    stdio: "ignore",
  });
} catch {
  // cargo not on PATH or lock not present — the tauri CI build regenerates it.
}

git(["add", "-A"]);
git(["commit", "-m", `chore: release ${tag}`]);
git(["tag", tag]);
step(`Committed and tagged ${tag}`);

step(`Pushing main + ${tag} to origin (this triggers the release workflow)`);
git(["push", "origin", "main", tag]);

console.log(`\x1b[32m✓ ${tag} pushed. Watch the build: gh run watch\x1b[0m`);
