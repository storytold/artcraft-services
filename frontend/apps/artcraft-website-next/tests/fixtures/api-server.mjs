import { createServer } from "node:http";

// Next's server-side metadata requests cannot be intercepted by page.route.
// This local-only fixture has no upstream and never connects to a database.
const CDN = "https://media-fixture.invalid";
const publicMedia = {
  m_fixture_image: {
    token: "m_fixture_image", media_class: "image",
    maybe_creator_user: { username: "fixture_artist", display_name: "Fixture Artist" },
    media_links: { cdn_url: `${CDN}/asset.png?fixture=1` },
  },
  m_fixture_video: {
    token: "m_fixture_video", media_class: "video",
    media_links: { cdn_url: `${CDN}/asset.mp4`, maybe_video_previews: {
      still: `${CDN}/still.jpg`, still_thumbnail_template: `${CDN}/still-{WIDTH}.jpg`,
    } },
  },
  m_fixture_audio: {
    token: "m_fixture_audio", media_class: "audio",
    media_links: { cdn_url: `${CDN}/asset.wav` },
  },
};

createServer((request, response) => {
  response.setHeader("Content-Type", "application/json");
  if (request.url === "/health") {
    response.end(JSON.stringify({ success: true }));
    return;
  }
  // A regression forwarding visitor credentials must not produce public cards.
  if (request.headers.cookie || request.headers.authorization || request.headers.session) {
    response.writeHead(403).end(JSON.stringify({ success: false }));
    return;
  }
  const token = request.url?.match(/^\/v1\/media_files\/file\/([^/?]+)$/)?.[1];
  const media = publicMedia[token];
  response.writeHead(media ? 200 : 404).end(JSON.stringify(media
    ? { success: true, media_file: media }
    : { success: false }));
}).listen(4203, "127.0.0.1");
