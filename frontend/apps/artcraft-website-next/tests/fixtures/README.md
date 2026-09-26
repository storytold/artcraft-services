# Media fixtures

`clip.mp4` is a generated one-second solid-blue H.264 video with no audio:

```sh
ffmpeg -f lavfi -i color=c=0x3366cc:s=64x64:d=1 -c:v libx264 -pix_fmt yuv420p -movflags +faststart clip.mp4
```

The tests construct their tiny PNG, sine-wave WAV, triangle GLB, and 27-point
Gaussian splat fixtures locally. All Storyteller API and media CDN requests are
intercepted; no production API or database is used. Run `npm run test:media` with
Google Chrome installed. Playwright uses Chrome, not a downloaded browser.
