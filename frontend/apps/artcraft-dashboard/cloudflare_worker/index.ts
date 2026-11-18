// Cloudflare Worker for dashboard.artcraft.team.
//
// The dashboard is a static Vite SPA, so the Worker serves `dist/` and only steps
// in for `/api/*`. The dashboard calls the API with `credentials: "include"`, and
// api.storyteller.ai is a different site than dashboard.artcraft.team, so the
// session cookie would be third-party and browsers would drop it. Proxying keeps
// every API call first-party — the same thing Netlify's `/api/*` rewrite did.

const API_ORIGIN = "https://api.storyteller.ai";
const API_PREFIX = "/api";

// Minimal shape of the static-assets binding declared in wrangler.jsonc. Wrangler
// bundles this file itself, so no worker runtime types are needed here.
interface AssetsBinding {
  fetch(request: Request): Promise<Response>;
}

interface Env {
  ASSETS: AssetsBinding;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (url.pathname === API_PREFIX || url.pathname.startsWith(`${API_PREFIX}/`)) {
      return fetch(proxyToApi(request, url));
    }

    return env.ASSETS.fetch(request);
  },
};

function proxyToApi(request: Request, url: URL): Request {
  const upstream = new URL(API_ORIGIN);
  upstream.pathname = url.pathname.slice(API_PREFIX.length) || "/";
  upstream.search = url.search;

  return new Request(upstream, request);
}
