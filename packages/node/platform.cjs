'use strict';
const { release } = require('node:os');
const targets = Object.freeze(['darwin-arm64', 'darwin-x64', 'linux-arm64-gnu', 'linux-x64-gnu', 'win32-arm64', 'win32-ia32', 'win32-x64']);
function selectTarget({ platform = process.platform, arch = process.arch, osRelease = release(), report = () => process.report.getReport(), nodeMajor = Number(process.versions.node.split('.')[0]) } = {}) {
  let target = `${platform}-${arch}`;
  if (platform === 'linux') {
    const glibc = report()?.header?.glibcVersionRuntime;
    if (!glibc) throw new Error('Rusty Robot prebuilt Linux binaries require glibc and X11; musl/Alpine is not supported by this package.');
    const [major, minor] = glibc.split('.').map(Number);
    if (major < 2 || (major === 2 && minor < 35)) throw new Error(`Rusty Robot requires glibc 2.35 or newer; found ${glibc}.`);
    target += '-gnu';
  }
  if (platform === 'darwin' && Number(osRelease.split('.')[0]) < 23) throw new Error('Rusty Robot requires macOS 14 or newer.');
  if (platform === 'win32' && arch === 'arm64' && nodeMajor < 22) throw new Error('Rusty Robot on native Windows ARM64 requires Node.js 22 or newer.');
  if (!targets.includes(target)) throw new Error(`Rusty Robot has no prebuilt binary for ${target}. Supported targets: ${targets.join(', ')}.`);
  return target;
}
module.exports = { targets, selectTarget };
