const { test } = require('node:test');
const assert = require('node:assert/strict');
const { encode } = require('./protocol.cjs');
test('observations preserve isolated UTF-16 event units without invalid Unicode JSON strings', () => {
  const frame = { text: 'Hello 👩‍💻 中文\n', events: [{ key: '\ud83d', data: '\udc69' }] };
  const decoded = JSON.parse(encode(frame));
  assert.equal(decoded.text, frame.text);
  assert.deepEqual(decoded.events, [{ key: { utf16CodeUnits: [0xd83d] }, data: { utf16CodeUnits: [0xdc69] } }]);
});
test('an incomplete editor value remains a mismatch, never repaired into passing text', () => {
  const decoded = JSON.parse(encode({ text: 'a\ud83d' }));
  assert.deepEqual(decoded.text, { utf16CodeUnits: [97, 0xd83d] });
  assert.notEqual(decoded.text, 'a�');
  assert.equal(JSON.parse(encode({ text: 'a🚀' })).text, 'a🚀');
});
