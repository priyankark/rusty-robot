// A real Electron window receiving OS events. No synthetic input or RobotJS.
const { app, BrowserWindow, screen } = require('electron');
const readline = require('node:readline');
const path = require('node:path');
const net = require('node:net');
app.commandLine.appendSwitch('force-color-profile', 'srgb');
app.commandLine.appendSwitch('force-raster-color-profile', 'srgb');
let window;
const send = value => process.stdout.write(JSON.stringify(value) + '\n');
app.whenReady().then(async () => {
  const displays = screen.getAllDisplays();
  const display = process.env.RUSTY_ROBOT_QA_SECONDARY
    ? displays.find(d => d.bounds.x < 0) : screen.getPrimaryDisplay();
  if (!display) throw new Error('Negative-origin secondary display required');
  window = new BrowserWindow({
    x: display.bounds.x + 60, y: display.bounds.y + 60,
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
      if (command.phase) await window.webContents.executeJavaScript(
        `document.querySelector('#phase').textContent = ${JSON.stringify(command.phase)}`);
      if (command.focus) {
        window.focus(); app.focus({ steal: true });
        window.focusOnWebView(); window.webContents.focus();
      }
      const state = await window.webContents.executeJavaScript('observe()');
      send({ ...state, nativeFocus: window.isFocused(), rendererFocus: window.webContents.isFocused() });
    } catch (error) { send({ error: String(error) }); }
  });
}).catch(error => { send({ error: String(error) }); app.quit(); });
app.on('window-all-closed', () => app.quit());
