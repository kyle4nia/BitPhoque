/**
 * bitphoque_qr.js -- QR code SVG generator for BitPhoque wallet + site.
 * Uses vendored qrcode-generator (MIT, Kazuhiko Arase).
 *
 * Usage:
 *   const { qrSvg } = await import('./bitphoque_qr.js');
 *   container.innerHTML = qrSvg('kaspa:qr...', 120);
 *                                 text       size(px)
 */

import qrcode from './vendor/qrcode-generator.mjs';

/**
 * Generate an inline SVG string for the given text.
 * @param {string} text   - data to encode
 * @param {number} [size] - desired width/height in px (default 120)
 * @returns {string} raw SVG markup (no wrapper tags needed)
 */
export function qrSvg(text, size = 120) {
  // typeNumber 0 = auto-detect; 'L' = low error correction (smallest)
  const qr = qrcode(0, 'L');
  qr.addData(text);
  qr.make();

  const count = qr.getModuleCount();
  const cell  = size / count;
  let paths = '';

  for (let row = 0; row < count; row++) {
    for (let col = 0; col < count; col++) {
      if (qr.isDark(row, col)) {
        const x = (col * cell).toFixed(2);
        const y = (row * cell).toFixed(2);
        const w = cell.toFixed(2);
        paths += `<rect x="${x}" y="${y}" width="${w}" height="${w}"/>`;
      }
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}" shape-rendering="crispEdges">
  <rect width="${size}" height="${size}" fill="#fff"/>
  <g fill="#111110">${paths}</g>
</svg>`;
}
