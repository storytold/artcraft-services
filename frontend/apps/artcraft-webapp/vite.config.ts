/// <reference types='vitest' />
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import { nxViteTsPaths } from '@nx/vite/plugins/nx-tsconfig-paths.plugin';

// Dev-only CDN proxy. Media assets (splats, GLBs, images) are fetched with
// fetch(), but the CDNs only send CORS headers for the production origin —
// not localhost. The web adapter's fetchAsset rewrites absolute CDN URLs to
// `/__cdn/<host>/<path>` in DEV; this middleware forwards them server-side
// (no browser CORS) to whatever host is embedded in the path.
//
// NB: This must be a middleware, not a `server.proxy` entry. The previous
// proxy config used a `router` option to pick the target host from the path,
// but `router` is an http-proxy-middleware feature — Vite's http-proxy
// silently ignores it, so every request went to the hardcoded default target
// and any asset on a different host (e.g. the dev R2 bucket) came back 403.
//
// Only the asset hosts the app actually loads from may be proxied; the dev
// server would otherwise fetch any host (including private addresses) on
// behalf of whoever can reach it.
const CDN_HOST_SUFFIXES = [
  'fakeyou.com',
  'storyteller.ai',
  'getartcraft.com',
  'artcraft.ai',
  'r2.dev',
  'storage.googleapis.com',
];
const CDN_PROXY_TIMEOUT_MS = 5 * 60_000;

function isAllowedCdnHost(host: string): boolean {
  const hostname = host.replace(/:\d+$/, '').toLowerCase();
  return CDN_HOST_SUFFIXES.some(
    (suffix) => hostname === suffix || hostname.endsWith(`.${suffix}`),
  );
}

function cdnProxyPlugin() {
  return {
    name: 'cdn-proxy',
    configureServer(server: {
      middlewares: {
        use: (
          route: string,
          handler: (
            req: import('http').IncomingMessage,
            res: import('http').ServerResponse,
          ) => void,
        ) => void;
      };
    }) {
      server.middlewares.use('/__cdn', async (req, res) => {
        try {
          // Connect strips the `/__cdn` mount prefix from req.url.
          const match = (req.url ?? '').match(/^\/([^/?]+)(\/[^?]*)?(\?.*)?$/);
          if (!match) {
            res.statusCode = 400;
            res.end('Bad /__cdn path; expected /__cdn/<host>/<path>');
            return;
          }
          const [, host, assetPath = '/', query = ''] = match;
          if (!isAllowedCdnHost(host)) {
            res.statusCode = 403;
            res.end('Host not allowed through /__cdn');
            return;
          }
          // Abort the upstream fetch when the browser goes away or the
          // transfer stalls, and never follow redirects off the allowlist.
          const controller = new AbortController();
          const timer = setTimeout(() => controller.abort(), CDN_PROXY_TIMEOUT_MS);
          res.on('close', () => controller.abort());
          const upstream = await fetch(`https://${host}${assetPath}${query}`, {
            method: req.method === 'HEAD' ? 'HEAD' : 'GET',
            headers: req.headers.range
              ? { range: String(req.headers.range) }
              : undefined,
            redirect: 'manual',
            signal: controller.signal,
          }).finally(() => clearTimeout(timer));
          res.statusCode = upstream.status;
          for (const header of [
            'content-type',
            'content-length',
            'accept-ranges',
            'content-range',
            'etag',
            'cache-control',
          ]) {
            const value = upstream.headers.get(header);
            if (value) res.setHeader(header, value);
          }
          // Stream instead of buffering: a large or highly compressed asset
          // no longer has to fit in the dev server's memory.
          if (upstream.body) {
            for await (const chunk of upstream.body) {
              if (!res.write(chunk)) {
                await new Promise((resolve) => res.once('drain', resolve));
              }
            }
          }
          res.end();
        } catch (e) {
          res.statusCode = 502;
          res.end(`CDN proxy error: ${e}`);
        }
      });
    },
  };
}

export default defineConfig(() => ({
  root: __dirname,
  cacheDir: '../../node_modules/.vite/apps/artcraft-webapp',
  server:{
    port: 4201,
    host: 'localhost',
    proxy: {
      // Forward API calls to production API to avoid CORS during local dev
      '/v1': {
        target: 'https://api.storyteller.ai',
        changeOrigin: true,
        secure: true,
        headers: {
          Origin: 'https://api.storyteller.ai',
        },
      },
      // NB: `/__cdn` asset proxying lives in `cdnProxyPlugin` above — it
      // needs per-request host routing, which `server.proxy` can't do.
    },
  },
  preview:{
    port: 4301,
    host: 'localhost',
  },
  plugins: [cdnProxyPlugin(), nxViteTsPaths(), react(), wasm(), topLevelAwait()],
  // Uncomment this if you are using workers.
  // worker: {
  //  plugins: [ nxViteTsPaths() ],
  // },
  build: {
    outDir: './dist',
    emptyOutDir: true,
    reportCompressedSize: true,
    commonjsOptions: {
      transformMixedEsModules: true,
    },
  },
  test: {
    watch: false,
    globals: true,
    environment: 'jsdom',
    include: ['{src,tests}/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    reporters: ['default'],
    coverage: {
      reportsDirectory: './test-output/vitest/coverage',
      provider: 'v8' as const,
    }
  },
}));
