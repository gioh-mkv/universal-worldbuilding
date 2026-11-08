#!/usr/bin/env node
// Atualiza a versão em package.json, tauri.conf.json, Cargo.toml e Cargo.lock.
// Uso: node scripts/bump-version.mjs 0.2.0   (ou 0.2.0-beta.1)

import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? "")) {
  console.error("Uso: node scripts/bump-version.mjs X.Y.Z   (ex.: 0.2.0 ou 0.2.0-beta.1)");
  process.exit(1);
}

// 1) package.json e package-lock.json
execSync(`npm version ${version} --no-git-tag-version --allow-same-version`, {
  stdio: "inherit",
});

// 2) src-tauri/tauri.conf.json
const confPath = "src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.version = version;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

// 3) src-tauri/Cargo.toml (a primeira linha "version = ..." é a do [package])
const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
if (!/^version\s*=\s*"[^"]*"/m.test(cargo)) {
  console.error("Não encontrei a linha version = \"...\" em src-tauri/Cargo.toml");
  process.exit(1);
}
writeFileSync(cargoPath, cargo.replace(/^version\s*=\s*"[^"]*"/m, `version = "${version}"`));

// 4) Cargo.lock (o cargo reescreve a versão do próprio pacote)
execSync("cargo metadata --format-version 1 --manifest-path src-tauri/Cargo.toml", {
  stdio: "ignore",
});

console.log(`\nVersão atualizada para ${version}:`);
execSync("git diff --stat", { stdio: "inherit" });