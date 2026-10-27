const { test } = require('node:test');
const assert = require('node:assert/strict');
const { selectTarget } = require('./platform.cjs');
test('platform selection rejects unsupported operating systems and ABI baselines before loading', () => {
  for (const arch of ['arm64', 'x64']) {
    assert.equal(selectTarget({ platform: 'darwin', arch, osRelease: '23.0' }), `darwin-${arch}`);
    assert.equal(selectTarget({ platform: 'linux', arch, report: () => ({ header: { glibcVersionRuntime: '2.35' } }) }), `linux-${arch}-gnu`);
  }
  for (const arch of ['arm64', 'ia32', 'x64']) assert.equal(selectTarget({ platform: 'win32', arch, nodeMajor: 22 }), `win32-${arch}`);
  assert.throws(() => selectTarget({ platform: 'darwin', arch: 'x64', osRelease: '22.0' }), /macOS 14/);
  assert.throws(() => selectTarget({ platform: 'linux', arch: 'x64', report: () => ({ header: {} }) }), /musl/);
  assert.throws(() => selectTarget({ platform: 'linux', arch: 'x64', report: () => ({ header: { glibcVersionRuntime: '2.34' } }) }), /glibc 2.35/);
  assert.throws(() => selectTarget({ platform: 'win32', arch: 'arm64', nodeMajor: 18 }), /Node.js 22/);
  assert.throws(() => selectTarget({ platform: 'freebsd', arch: 'x64' }), /no prebuilt/);
});
