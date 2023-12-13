// @ts-check
import { fileURLToPath, pathToFileURL } from "node:url";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { Writable } from "node:stream";
import { createWriteStream } from "node:fs";
import { $ } from "execa";

/**
 *
 * @param {import('node:fs').PathLike} destRaw
 * @param {string} [tag]
 * @param {string} [osRaw]
 * @param {string} [archRaw]
 */
export async function typstMetaInstall(
  destRaw = process.cwd(),
  tag = undefined,
  osRaw = process.platform,
  archRaw = process.arch
) {
  const destPath =
    destRaw instanceof URL ? fileURLToPath(destRaw) : `${destRaw}`;
  const destFileURL =
    destRaw instanceof URL ? destRaw : pathToFileURL(`${destRaw}`);
  destFileURL.pathname.endsWith("/") || (destFileURL.pathname += "/");

  if (tag === undefined) {
    const response = await fetch(
      `https://ungh.cc/repos/typst/typst/releases/latest`
    );
    const json = await response.json();
    tag = json.release.tag;
  }

  const os = {
    Windows: "win32",
    Linux: "linux",
    macOS: "darwin",
    win32: "win32",
    linux: "linux",
    darwin: "darwin",
  }[osRaw];
  const arch = {
    X64: "x64",
    ARM64: "arm64",
    ARM: "arm",
    x64: "x64",
    arm64: "arm64",
    arm: "arm",
  }[archRaw];

  const archive = {
    "darwin,arm64": "typst-aarch64-apple-darwin.tar.xz",
    "linux,x64": "typst-x86_64-unknown-linux-musl.tar.xz",
    "linux,arm": "typst-armv7-unknown-linux-musleabi.tar.xz",
    "darwin,x64": "typst-x86_64-apple-darwin.tar.xz",
    "win32,x64": "typst-x86_64-pc-windows-msvc.zip",
    "linux,arm64": "typst-aarch64-unknown-linux-musl.tar.xz",
  }[[os, arch].toString()];

  const folder = {
    "darwin,arm64": "typst-aarch64-apple-darwin",
    "linux,x64": "typst-x86_64-unknown-linux-musl",
    "linux,arm": "typst-armv7-unknown-linux-musleabi",
    "darwin,x64": "typst-x86_64-apple-darwin",
    "win32,x64": "typst-x86_64-pc-windows-msvc",
    "linux,arm64": "typst-aarch64-unknown-linux-musl",
  }[[os, arch].toString()];

  const response = await fetch(
    `https://github.com/typst/typst/releases/download/${tag}/${archive}`
  );
  await mkdir(destFileURL, { recursive: true });
  const writable = Writable.toWeb(
    createWriteStream(new URL(archive, destFileURL))
  );
  if (!response.body) throw new DOMException("No body", "NotSupportedError");
  await response.body.pipeTo(writable);

  if (archive.endsWith(".zip")) {
    await $`powershell Expand-Archive -Path ${fileURLToPath(
      new URL(archive, destFileURL)
    )} -DestinationPath ${destPath} -Force`;
  } else if (archive.endsWith(".tar.xz")) {
    await $`tar -xJf ${fileURLToPath(
      new URL(archive, destFileURL)
    )} -C ${destPath}`;
  } else {
    throw new DOMException(
      `Unsupported archive format: ${archive}`,
      "NotSupportedError"
    );
  }

  await rm(new URL(archive, destFileURL), { force: true });
  await mkdir(new URL("./bin/", destFileURL), { recursive: true });
  await rename(
    new URL(`./${folder}/typst`, destFileURL),
    new URL("./bin/typst", destFileURL)
  );
  await rm(new URL(`./${folder}`, destFileURL), {
    recursive: true,
    force: true,
  });
}
