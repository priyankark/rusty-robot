'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { fork } = require('node:child_process');

function needsIsolation(platform = process.platform, release = os.release()) {
  return platform === 'darwin' && Number.parseInt(release, 10) >= 27;
}
function failure(message, code = 'ERR_RUSTY_MEDIA_WORKER') {
  return Object.assign(new Error(message), { code });
}

// Options are private test seams, not environment overrides or public API.
function createCaptureClient(bindingPath, { timeoutMs = 15000, idleMs = 5000,
  temporaryRoot = os.tmpdir(), executable = process.execPath } = {}) {
  let current = null, sequence = 0;

  function remove(state) {
    if (state.starting || !state.directory) return;
    try { fs.rmSync(state.directory, { recursive: true, force: true }); } catch { /* Best effort on OS teardown. */ }
  }
  function close(state, error) {
    if (state.closed) return;
    state.closed = true;
    if (current === state) current = null;
    clearTimeout(state.idle);
    process.removeListener('exit', state.onExit);
    for (const job of state.jobs) {
      clearTimeout(job.timer);
      job.reject(error || failure('Capture worker closed'));
    }
    state.jobs.length = 0;
    if (state.child?.pid && state.child.exitCode === null && state.child.signalCode === null) {
      state.child.once('exit', () => remove(state));
      state.child.kill('SIGKILL'); // Only this package-owned worker, never another capture client.
    } else remove(state);
  }
  function dispatch(state) {
    if (state.closed || !state.ready || state.active || !state.jobs.length) return;
    state.active = state.jobs[0];
    state.child.ref();
    state.child.channel?.ref();
    state.child.send({ type: state.active.type, id: state.active.id, args: state.active.args }, error => {
      if (error) close(state, failure(`Capture worker IPC failed: ${error.message}`));
    });
  }
  async function start(state) {
    try {
      state.directory = await fs.promises.mkdtemp(path.join(temporaryRoot, 'rusty-robot-capture-'));
      await fs.promises.chmod(state.directory, 0o700);
      if (state.closed) return;
      const runtime = path.join(state.directory, process.platform === 'win32' ? 'node.exe' : 'node');
      // COPYFILE_FICLONE uses an APFS clone, with an ordinary byte copy where unsupported.
      await fs.promises.copyFile(await fs.promises.realpath(executable), runtime, fs.constants.COPYFILE_FICLONE);
      await fs.promises.chmod(runtime, 0o700);
      if (state.closed) return;
      state.child = fork(path.join(__dirname, 'capture-worker.cjs'), [bindingPath, state.directory, String(timeoutMs)], {
        execPath: runtime, execArgv: [], serialization: 'advanced',
        stdio: ['ignore', 'ignore', 'ignore', 'ipc'],
        env: { ...process.env, NODE_OPTIONS: '', ELECTRON_RUN_AS_NODE: '1' },
      });
      state.child.on('error', error => close(state, failure(`Capture worker failed: ${error.message}`)));
      state.child.on('exit', (code, signal) => close(state, failure(`Capture worker exited (${signal || code})`)));
      state.child.on('disconnect', () => close(state, failure('Capture worker disconnected')));
      state.child.on('message', message => {
        if (state.closed) return;
        if (message?.type === 'startup-error') {
          close(state, failure(`Capture worker could not load Rusty Robot: ${message.error.message}`, message.error.code));
        } else if (message?.type === 'ready') {
          state.ready = true;
          dispatch(state);
        } else if (state.active && message?.id === state.active.id) {
          const job = state.jobs.shift();
          state.active = null;
          clearTimeout(job.timer);
          if (message.error) job.reject(failure(message.error.message, message.error.code));
          else job.resolve(message.frame);
          if (state.jobs.length) dispatch(state);
          else {
            state.child.unref();
            state.child.channel?.unref();
            state.idle = setTimeout(() => close(state), idleMs);
            state.idle.unref();
          }
        }
      });
    } catch (error) {
      close(state, failure(`Capture worker could not start: ${error.message}`));
    } finally {
      state.starting = false;
      if (state.closed) remove(state);
    }
  }
  return {
    capture(args, type = 'capture') {
      if (!current) {
        const state = current = { jobs: [], closed: false, starting: true };
        state.onExit = () => {
          state.closed = true;
          state.child?.kill('SIGKILL');
          remove(state);
        };
        process.once('exit', state.onExit);
        void start(state);
      }
      const state = current;
      clearTimeout(state.idle);
      return new Promise((resolve, reject) => {
        const job = { id: ++sequence, type, args, resolve, reject };
        job.timer = setTimeout(() => close(state,
          failure(`Rusty Robot ${type} exceeded ${timeoutMs} ms; worker terminated (no replay)`, 'ERR_RUSTY_MEDIA_TIMEOUT')), timeoutMs);
        state.jobs.push(job);
        dispatch(state);
      });
    },
    close() { if (current) close(current); },
  };
}

function isolateCapture(native, bindingPath, options) {
  const client = createCaptureClient(bindingPath, options);
  let pending = 0;
  function media(operation) {
    if (pending >= 2) throw failure('Rust frame queue is full; retry after the current frame', 'GenericFailure');
    pending++;
    try { return Promise.resolve(operation()).finally(() => pending--); }
    catch (error) { pending--; throw error; }
  }
  return { ...native,
    captureFrame(width, quality, previous, force, format) {
      native.validateCaptureFrame(width, quality, previous, force, format);
      return media(() => client.capture([width, quality, previous, force, format]));
    },
    resizeImage(input, width, height, quality, format) {
      native.validateResizeImage(input, width, height, quality, format);
      // Snapshot before returning: callers may immediately reuse/mutate their buffer.
      return media(() => client.capture([Buffer.from(input), width, height, quality, format], 'resize'));
    },
  };
}
module.exports = { needsIsolation, createCaptureClient, isolateCapture };
