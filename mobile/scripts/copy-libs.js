// Copy pdf.js (the PDF renderer, Apache-2.0) from node_modules into www/lib/pdfjs, where
// www/index.html loads it. Run by `npm run libs` (and by the APK workflow) before `cap sync`.
'use strict';
var fs = require('fs');
var path = require('path');
var root = path.join(__dirname, '..');
var src = path.join(root, 'node_modules', 'pdfjs-dist');
var dst = path.join(root, 'www', 'lib', 'pdfjs');

function copyDir(from, to) {
  fs.mkdirSync(to, { recursive: true });
  fs.readdirSync(from).forEach(function (name) {
    var a = path.join(from, name);
    var b = path.join(to, name);
    if (fs.statSync(a).isDirectory()) copyDir(a, b);
    else fs.copyFileSync(a, b);
  });
}

fs.rmSync(dst, { recursive: true, force: true });
fs.mkdirSync(dst, { recursive: true });
// The legacy build runs on older Android System WebViews too.
['pdf.min.mjs', 'pdf.worker.min.mjs'].forEach(function (f) {
  fs.copyFileSync(path.join(src, 'legacy', 'build', f), path.join(dst, f));
});
// Character maps (CJK and other non-embedded encodings), the standard 14 fonts, the JPEG 2000 /
// colour decoders (WebAssembly) and colour profiles that pdf.js loads on demand.
['cmaps', 'standard_fonts', 'wasm', 'iccs'].forEach(function (d) {
  if (fs.existsSync(path.join(src, d))) copyDir(path.join(src, d), path.join(dst, d));
});
fs.copyFileSync(path.join(src, 'LICENSE'), path.join(dst, 'LICENSE'));
console.log('pdf.js ' + require(path.join(src, 'package.json')).version + ' copied to www/lib/pdfjs');
