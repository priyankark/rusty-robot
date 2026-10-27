#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const { copyFileSync } = require('node:fs');
const { resolve, join } = require('node:path');
const root = resolve(__dirname, '..');
const target = process.env.CARGO_BUILD_TARGET;
const result = spawnSync('cargo', ['build', '--release', '--locked', '-p', 'rusty-robot-node', ...(target ? ['--target', target] : [])], { cwd: root, stdio: 'inherit', shell: false });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status || 1);
const library = process.platform === 'win32' ? 'rusty_robot_node.dll' : process.platform === 'darwin' ? 'librusty_robot_node.dylib' : 'librusty_robot_node.so';
copyFileSync(join(root, 'target', target || '', 'release', library), join(root, 'target/rusty-robot.node'));
