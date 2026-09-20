#!/usr/bin/env node
import { existsSync, lstatSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const WARNING_BYTES = 25_000_000;
export const LIMIT_BYTES = 30_000_000;

export function pathBytes(path) {
  const stat = lstatSync(path);
  if (stat.isSymbolicLink()) return 0;
  if (stat.isFile()) return stat.size;
  if (!stat.isDirectory()) return 0;
  return readdirSync(path).reduce((total, entry) => total + pathBytes(join(path, entry)), 0);
}

export function sizeState(bytes) {
  if (bytes > LIMIT_BYTES) return 'FAIL';
  if (bytes > WARNING_BYTES) return 'WARN';
  return 'OK';
}

function children(path) {
  return existsSync(path) ? readdirSync(path, { withFileTypes: true }) : [];
}

function matchingFiles(path, suffixes) {
  if (!existsSync(path)) return [];
  return children(path).flatMap((entry) => {
    const child = join(path, entry.name);
    if (entry.isDirectory()) return matchingFiles(child, suffixes);
    return suffixes.some((suffix) => entry.name.endsWith(suffix)) ? [child] : [];
  });
}

function knownOutputs(root) {
  const outputs = [];
  const target = join(root, 'target');
  const bundleRoots = [join(target, 'release', 'bundle')];
  for (const entry of children(target).filter((item) => item.isDirectory() && item.name !== 'release')) {
    bundleRoots.push(join(target, entry.name, 'release', 'bundle'));
  }
  for (const release of bundleRoots) {
    outputs.push(...matchingFiles(join(release, 'dmg'), ['.dmg']));
    outputs.push(
      ...children(join(release, 'macos'))
        .filter((item) => item.isDirectory() && item.name.endsWith('.app'))
        .map((item) => join(release, 'macos', item.name)),
    );
  }
  outputs.push(...matchingFiles(join(target, 'release', 'bundle', 'nsis'), ['.exe']));
  const windowsExecutable = join(target, 'release', 'vidya-app.exe');
  if (existsSync(windowsExecutable)) outputs.push(windowsExecutable);
  outputs.push(
    ...matchingFiles(join(root, 'src-tauri', 'gen', 'android', 'app', 'build', 'outputs'), ['.apk', '.aab']),
  );
  return [...new Set(outputs)];
}

function bundledResourceBytes(root) {
  const configPath = join(root, 'src-tauri', 'tauri.conf.json');
  if (!existsSync(configPath)) return 0;
  const resources = JSON.parse(readFileSync(configPath, 'utf8')).bundle?.resources;
  if (!resources) return 0;
  const relativePaths = Array.isArray(resources) ? resources : Object.keys(resources);
  return relativePaths.reduce((total, relativePath) => {
    const path = resolve(join(root, 'src-tauri'), relativePath);
    return total + (existsSync(path) ? pathBytes(path) : 0);
  }, 0);
}

function kind(path) {
  if (path.endsWith('.app')) return 'macOS installed';
  if (path.endsWith('.dmg')) return 'macOS download';
  if (path.endsWith('.apk')) return 'Android APK';
  if (path.endsWith('.aab')) return 'Android AAB';
  if (path.endsWith('-setup.exe')) return 'Windows installer';
  if (path.endsWith('.exe')) return 'Windows installed';
  return 'Input';
}

export function measure(paths) {
  return paths.map((path) => {
    const absolute = resolve(path);
    const bytes = pathBytes(absolute);
    return { path: absolute, name: basename(absolute), kind: kind(absolute), bytes, state: sizeState(bytes) };
  });
}

function main() {
  const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
  const paths = process.argv.length > 2 ? process.argv.slice(2) : knownOutputs(root);
  if (paths.length === 0) {
    console.log('size: no known build outputs found');
    writeFileSync(join(root, 'size-report.json'), `${JSON.stringify({ files: [] }, null, 2)}\n`);
    return;
  }

  const rows = measure(paths);
  const resourceBytes = bundledResourceBytes(root);
  for (const row of rows.filter(({ kind: rowKind }) => rowKind === 'Windows installed')) {
    row.bytes += resourceBytes;
    row.state = sizeState(row.bytes);
  }
  console.log('State  MB     Kind                 File');
  for (const row of rows) {
    console.log(
      `${row.state.padEnd(5)}  ${(row.bytes / 1_000_000).toFixed(2).padStart(5)}  ${row.kind.padEnd(20)} ${row.path}`,
    );
  }
  writeFileSync(join(root, 'size-report.json'), `${JSON.stringify({ files: rows }, null, 2)}\n`);
  if (rows.some(({ state }) => state === 'FAIL')) process.exitCode = 1;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
