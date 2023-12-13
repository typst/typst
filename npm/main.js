#!/usr/bin/env node
// @ts-check
import { existsSync } from "node:fs";
import { $ } from "execa";
import { typstMetaInstall } from "./utils.js";
import { fileURLToPath, pathToFileURL } from "node:url";
import { readFile, writeFile } from "node:fs/promises";
import { relative } from "node:path";

const package_ = JSON.parse(
  await readFile(new URL("./package.json", import.meta.url), "utf8")
);
const tag = `v${package_.version.match(/^\d+\.\d+\.\d+/)[0]}`;

const ext = process.platform === "win32" ? ".exe" : "";
const typst = fileURLToPath(new URL(`./bin/typst${ext}`, import.meta.url));

if (!existsSync(typst)) {
  await typstMetaInstall(undefined, tag);
}

const { exitCode, signal } = await $({
  stdio: "inherit",
  reject: false,
})`${typst} ${process.argv.slice(2)}`;
if (signal) process.kill(process.pid, signal);
process.exit(exitCode ?? 100);
