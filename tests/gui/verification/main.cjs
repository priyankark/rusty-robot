// A real Electron window receiving OS events. No synthetic input or RobotJS.
const { app, BrowserWindow, screen, clipboard } = require('electron');
const readline = require('node:readline');
const path = require('node:path');
const net = require('node:net');
app.commandLine.appendSwitch('force-color-profile', 'srgb');
app.commandLine.appendSwitch('force-raster-color-profile', 'srgb');
let window;
let decoy;
let clipboardLease;
let clipboardGeneration = 0;
// Fixture-only text clipboard adapter. Reject richer formats before changing them.
// Production hosts should provide their own revision-aware clipboard implementation.
function textSnapshot() {
  const formats = clipboard.availableFormats();
  if (formats.some(f => f !== 'text/plain')) throw new Error('Fixture only preserves plain-text clipboards');
  return { text: clipboard.readText(), empty: formats.length === 0 };
}
function restoreClipboard() {
  if (!clipboardLease) return false;
  const lease = clipboardLease; clipboardLease = undefined;
  const current = textSnapshot();
  if (current.text !== lease.text || current.empty) return false;
  if (lease.previous.empty) clipboard.clear(); else clipboard.writeText(lease.previous.text);
  return true;
}
const { encode } = require('./protocol.cjs');
const send = value => process.stdout.write(encode(value) + '\n');
app.whenReady().then(async () => {
  const displays = screen.getAllDisplays();
  const display = process.env.RUSTY_ROBOT_QA_SECONDARY
    ? displays.find(d => d.bounds.x < 0) : screen.getPrimaryDisplay();
  if (!display) throw new Error('Negative-origin secondary display required');
  window = new BrowserWindow({
    x: display.bounds.x + Math.min(60, Math.max(0, display.bounds.width - 1000)),
    y: display.bounds.y + Math.min(60, Math.max(0, display.bounds.height - 620)),
    width: 1000, height: 620, frame: false, resizable: false,
    alwaysOnTop: true, acceptFirstMouse: true,
    webPreferences: { backgroundThrottling: false },
  });
  await window.loadFile(path.join(__dirname, 'index.html'));
  window.show(); window.focus(); app.focus({ steal: true });
  window.focusOnWebView(); window.webContents.focus();
  const bounds = window.getContentBounds();
  const elements = await window.webContents.executeJavaScript('geometry()');
  send({ ready: true, bounds, elements });
  // This channel only reads observations and labels screenshots; input is native.
  readline.createInterface({ input: net.createConnection({
    host: '127.0.0.1', port: Number(process.env.RUSTY_ROBOT_QA_CONTROL_PORT),
  }).on('error', error => { send({ error: String(error) }); app.quit(); }) }).on('line', async line => {
    try {
      const command = JSON.parse(line);
      let extra = {};
      if (command.clipboardInstall !== undefined) {
        if (clipboardLease) throw new Error('Clipboard lease already active');
        const previous = textSnapshot();
        clipboard.writeText(command.clipboardInstall);
        clipboardLease = { previous, text: command.clipboardInstall, id: ++clipboardGeneration };
        extra.clipboardLease = clipboardLease.id;
      }
      if (command.clipboardRestore !== undefined) {
        if (!clipboardLease || clipboardLease.id !== command.clipboardRestore) throw new Error('Wrong clipboard lease');
        extra.clipboardRestored = restoreClipboard();
      }
      if (command.decoy === true) {
        decoy = new BrowserWindow({width:300,height:200,alwaysOnTop:true});
        await decoy.loadURL('data:text/html,<title>Owned focus decoy</title>Focus decoy');
        decoy.show(); decoy.focus();
      }
      if (command.decoy === false && decoy) { decoy.close(); decoy = undefined; }
      if (command.autoindent !== undefined) await window.webContents.executeJavaScript(`window.autoindent = ${Boolean(command.autoindent)}`);
      if (command.phase) await window.webContents.executeJavaScript(
        `document.querySelector('#phase').textContent = ${JSON.stringify(command.phase)}`);
      if (command.focus) {
        window.focus(); app.focus({ steal: true });
        window.focusOnWebView(); window.webContents.focus();
      }
      const state = await window.webContents.executeJavaScript('observe()');
      send({ ...state, ...extra, nativeFocus: window.isFocused(), rendererFocus: window.webContents.isFocused() });
    } catch (error) { send({ error: String(error) }); }
  });
}).catch(error => { send({ error: String(error) }); app.quit(); });
app.on('window-all-closed', () => app.quit());
app.on('will-quit', () => { try { restoreClipboard(); } catch {} });
