import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import svgr from 'vite-plugin-svgr';
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { devPort } = require('./package.json');

export default defineConfig({
  // svgr: `import Icon from './assets/icon-yaiol-x.svg?react'` -> a real React component, built
  // from the .svg FILE. The icon stays editable in src/assets (open it in Inkscape, save, HMR
  // repaints) and nothing is ever pasted into a .jsx. This is also what makes `currentColor` work:
  // svgr inlines real <svg> elements into the page, so the icon inherits its button's colour --
  // an <img> renders the file as an isolated document that the page's CSS cannot reach.
  // ⚠ CLAUDE: the two `plugins` entries are BOTH required and ORDER MATTERS. vite-plugin-svgr
  // hands svgrOptions straight to @svgr/core with plugin-jsx as its ONLY default, so naming
  // plugin-jsx here is not redundant -- listing plugin-svgo alone would REPLACE the jsx step and
  // emit no component. svgo must run first: it strips the Inkscape cruft (sodipodi:namedview,
  // inkscape:* attributes) that would otherwise ride into the bundle on every save.
  plugins: [react(), svgr({
    svgrOptions: {
      plugins: ['@svgr/plugin-svgo', '@svgr/plugin-jsx'],
      svgoConfig: {
        plugins: [
          // ⚠ CLAUDE: removeViewBox MUST stay false. svgo's default preset drops viewBox when
          // width/height are present, and without it the icon cannot scale -- it would ignore the
          // catalog's `.btn > svg { width: 14px }` and paint at its authored size.
          { name: 'preset-default', params: { overrides: { removeViewBox: false } } },
          // Namespaces ids AND class names per file. Icons are inlined into ONE document, so two
          // files that both carry Inkscape's default `.cls-1` would otherwise fight over it and
          // silently recolour each other.
          'prefixIds',
        ],
      },
    },
  })],
  base: './',
  server: {
    port: devPort,
    strictPort: true, // fail if the port is taken — never silently grab another app's port
    open: false,
  },
  build: {
    chunkSizeWarningLimit: 2000, // Electron app - code-splitting has no benefit; silences expected warning
  },
});
