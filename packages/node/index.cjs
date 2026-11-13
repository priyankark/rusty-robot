'use strict';
const native = require('./native.cjs');
const { needsIsolation, isolateCapture } = require('./capture-client.cjs');
module.exports = needsIsolation() ? isolateCapture(native, require.resolve('./native.cjs')) : native;
