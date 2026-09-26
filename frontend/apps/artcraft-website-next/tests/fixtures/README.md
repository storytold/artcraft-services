# Media fixtures

`clip.mp4` is a generated one-second solid-blue H.264 video with no audio:

```sh
ffmpeg -f lavfi -i color=c=0x3366cc:s=64x64:d=1 -c:v libx264 -pix_fmt yuv420p -movflags +faststart clip.mp4
```

The tests construct their tiny PNG, sine-wave WAV, triangle GLB, and 27-point
Gaussian splat fixtures locally. All Storyteller API and media CDN requests are
intercepted in the browser; server-side social-card lookups use `api-server.mjs`
on `127.0.0.1:4203`, which has no upstream. No production API or database is used.
Run `npm run test:media` with Google Chrome installed and ports 4202 and 4203 free.
The suite starts its own Next server with the fixture API host, so it refuses to
reuse a running development server. Playwright uses installed Chrome.
