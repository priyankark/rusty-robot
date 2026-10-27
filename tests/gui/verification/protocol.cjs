// Windows key/input events can expose one UTF-16 surrogate at a time. Rust's
// serde_json rejects lone surrogates in JSON strings. Preserve those observations
// as raw code units instead of dropping the response or replacing received text.
function encode(value) {
  return JSON.stringify(value, (_key, item) => {
    if (typeof item === 'string' && !item.isWellFormed()) {
      return { utf16CodeUnits: Array.from({ length: item.length }, (_, i) => item.charCodeAt(i)) };
    }
    return item;
  });
}
module.exports = { encode };
