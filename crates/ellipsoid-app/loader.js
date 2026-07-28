// Progress reporting for the wasm download, wired up by trunk's
// `data-initializer` hook in index.html.
//
// # The percentage is real
//
// Worth stating, because the obvious worry is wrong. `total` is not
// Content-Length — trunk bakes the *uncompressed* size of the wasm into the
// generated loader at build time, and `current` accumulates bytes read from
// the decompressed stream. Both are in the same units, so the ratio holds even
// though the file arrives brotli-compressed and Content-Length is a much
// smaller number.
//
// The bundle is over 50 MB uncompressed (~7 MB on the wire), so this is not a
// nicety: without it the page is a spinner for however long the visitor's
// connection takes, with no way to tell progress from a hang.

const mb = (bytes) => (bytes / 1048576).toFixed(1);

export default function loader() {
  const overlay = document.getElementById('loading');
  const label = document.getElementById('loading-detail');
  const bar = document.getElementById('loading-bar');
  const fill = document.getElementById('loading-fill');

  return {
    onStart: () => {
      if (label) label.textContent = 'Downloading…';
    },

    onProgress: ({ current, total }) => {
      // Defensive: a missing or overshot total means the ratio is meaningless,
      // so fall back to the byte count and leave the bar out of it.
      const known = total > 0 && current <= total;
      if (label) {
        label.textContent = known
          ? `${mb(current)} / ${mb(total)} MB`
          : `${mb(current)} MB downloaded`;
      }
      if (!bar || !fill) return;
      bar.hidden = !known;
      if (known) fill.style.width = `${(current / total) * 100}%`;
    },

    onComplete: () => {
      if (label) label.textContent = 'Starting…';
    },

    onSuccess: () => {
      if (overlay) overlay.classList.add('hidden');
    },

    onFailure: (error) => {
      if (bar) bar.hidden = true;
      // A blank page tells the visitor nothing; a reason at least lets them
      // report it, or try a browser with WebGL2.
      if (label) {
        label.textContent = `Could not start: ${error}`;
        label.style.color = '#e06c5a';
      }
    },
  };
}
