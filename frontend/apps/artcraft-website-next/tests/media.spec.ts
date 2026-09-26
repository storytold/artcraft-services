import { expect, test, type Page, type Route } from "@playwright/test";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { corsMediaUrl, mediaKind, safeMediaUrl, type SharedMedia } from "../src/lib/media";

const API = "http://127.0.0.1:4203";
const CDN = "https://media-fixture.invalid";
const PNG = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1kAAAAASUVORK5CYII=", "base64");

test.beforeEach(async ({ page }) => {
  // All API calls are fulfilled locally, including navbar/session/referral
  // requests. Next's server-side metadata lookup uses the local fixture API.
  // These tests never reach production or a database.
  await page.route("**/v1/**", (route) => route.fulfill({ json: { success: true, logged_in: false } }));
  await page.route(`${CDN}/**`, (route) => route.fulfill({ body: PNG, contentType: "image/png" }));
});

test("direct shared image URL loads without login and survives refresh", async ({ page }) => {
  await mockMedia(page, fixture("image", "png"));
  await page.goto("/media/m_fixture_image");
  await expect(page.getByRole("heading", { name: "Fixture image", exact: true })).toBeVisible();
  await expect.poll(() => page.getByAltText("Fixture image").evaluate((image: HTMLImageElement) => image.naturalWidth)).toBe(1);
  await expect(page.getByText("@fixture_artist")).toBeVisible();
  await expect(page.getByText("1 × 1", { exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open in ArtCraft" })).toHaveAttribute("href", "https://app.getartcraft.com/media/m_fixture_image");
  await page.reload();
  await expect(page.getByAltText("Fixture image")).toBeVisible();
});

test("legacy query share links redirect to the canonical media page", async ({ page }) => {
  await mockMedia(page, fixture("image", "png"));
  await page.goto("/media?media=m_fixture_image");
  await expect(page).toHaveURL((url) => url.pathname === "/media/m_fixture_image");
  await expect(page.getByAltText("Fixture image")).toBeVisible();
});

test("social cards use public images and video stills without forwarding visitor credentials", async ({ request }) => {
  const headers = { "User-Agent": "Twitterbot", Cookie: "session=private-fixture", session: "private-fixture" };
  const image = await request.get("/media/m_fixture_image", { headers });
  const imageHtml = await image.text();
  expect(imageHtml).toContain('property="og:image" content="https://media-fixture.invalid/asset.png?fixture=1"');
  expect(imageHtml).toContain("Made with ArtCraft by Fixture Artist.");
  expect(imageHtml).toContain('name="robots" content="noindex, follow"');
  const video = await request.get("/media/m_fixture_video", { headers });
  expect(await video.text()).toContain('property="og:image" content="https://media-fixture.invalid/still-1200.jpg"');
  for (const token of ["m_fixture_audio", "m_private"]) {
    const response = await request.get(`/media/${token}`, { headers });
    expect(await response.text()).not.toContain('property="og:image" content="https://media-fixture.invalid/');
  }
});

test("prompt and reference media are restored without blocking the preview", async ({ page }) => {
  await mockMedia(page, { ...fixture("image", "png"), maybe_prompt_token: "p_fixture" });
  await page.route(`${API}/v1/prompts/p_fixture`, (route) => route.fulfill({ json: {
    success: true, prompt: { maybe_positive_prompt: "An ocean beneath the stars", maybe_model_type: "flux_pro",
      maybe_context_images: [{ media_token: "m_reference", semantic: "Reference image", media_links: { cdn_url: `${CDN}/reference.png` } }] },
  } }));
  await page.goto("/media/m_fixture_image");
  await expect(page.getByText("An ocean beneath the stars")).toBeVisible();
  await expect(page.getByRole("link").filter({ has: page.getByAltText("Reference image") })).toHaveAttribute("href", "/media/m_reference");
});

test("prompt failure leaves the image usable", async ({ page }) => {
  await mockMedia(page, { ...fixture("image", "png"), maybe_prompt_token: "p_private" });
  await page.route(`${API}/v1/prompts/**`, (route) => route.fulfill({ status: 403, json: { success: false } }));
  await page.goto("/media/m_fixture_image");
  await expect(page.getByText("Prompt details aren’t available for this creation.")).toBeVisible();
  await expect(page.getByAltText("Fixture image")).toBeVisible();
});

test("model/provider brands and video reference stills appear in creation details", async ({ page }) => {
  await mockMedia(page, { ...fixture("image", "png"), maybe_prompt_token: "p_fixture" });
  await page.route(`${API}/v1/prompts/p_fixture`, (route) => route.fulfill({ json: {
    success: true, prompt: { maybe_positive_prompt: "A forest", maybe_model_type: "worldlabs_gaussian", maybe_generation_provider: "worldlabs",
      maybe_context_images: [{ media_token: "m_reference_video", semantic: "Reference video", media_links: {
        cdn_url: `${CDN}/reference.mp4`, maybe_video_previews: { still: `${CDN}/still.jpg`, still_thumbnail_template: `${CDN}/still-{WIDTH}.jpg` },
      } }] },
  } }));
  await page.goto("/media/m_fixture_image");
  await expect(page.getByText("World Labs Marble", { exact: true })).toBeVisible();
  await expect(page.getByText("World Labs", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Media details").locator("img.themed-logo")).toHaveCount(2);
  await expect(page.getByAltText("Reference video")).toHaveAttribute("src", `${CDN}/still-256.jpg?cors=1`);
  await expect.poll(() => page.getByAltText("Reference video").evaluate((image: HTMLImageElement) => image.naturalWidth)).toBe(1);
  await expect(page.getByRole("link").filter({ has: page.getByAltText("Reference video") })).toHaveAttribute("href", "/media/m_reference_video");
});

test("video loads playable metadata and exposes native controls", async ({ page }) => {
  await mockMedia(page, fixture("video", "mp4"));
  await page.route(`${CDN}/asset.mp4*`, (route) => route.fulfill({ body: readFileSync(join(__dirname, "fixtures/clip.mp4")), contentType: "video/mp4" }));
  await page.goto("/media/m_fixture_video");
  const video = page.locator("video[aria-label='Fixture video']");
  await expect(video).toHaveAttribute("controls", "");
  await expect.poll(() => video.evaluate((el: HTMLVideoElement) => el.readyState)).toBeGreaterThanOrEqual(1);
  await video.evaluate((el: HTMLVideoElement) => { el.muted = true; return el.play(); });
  await expect.poll(() => video.evaluate((el: HTMLVideoElement) => el.currentTime)).toBeGreaterThan(0);
});

test("music loads in an audio player and shows its transcript", async ({ page }) => {
  await mockMedia(page, { ...fixture("audio", "wav"), maybe_text_transcript: "Fixture song lyrics" });
  await page.route(`${CDN}/asset.wav*`, (route) => serveBytes(route, wav(), "audio/wav"));
  await page.goto("/media/m_fixture_audio");
  const audio = page.locator("audio");
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.duration)).toBe(5);
  await page.getByRole("button", { name: "Play audio" }).click();
  await expect(page.getByRole("button", { name: "Pause audio" })).toBeVisible();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.currentTime)).toBeGreaterThan(0);
  await page.getByRole("button", { name: "Pause audio" }).click();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.paused)).toBe(true);
  await page.getByRole("slider", { name: "Seek audio" }).click();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.currentTime)).toBeCloseTo(2.5, 1);
  await expect(page.getByLabel("Elapsed time")).toHaveText("0:02");
  await page.getByRole("slider", { name: "Audio volume" }).click();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.volume)).toBe(0.5);
  await page.getByRole("button", { name: "Mute audio", exact: true }).click();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.muted)).toBe(true);
  await page.getByRole("button", { name: "Unmute audio", exact: true }).click();
  await expect.poll(() => audio.evaluate((el: HTMLAudioElement) => el.muted)).toBe(false);
  await expect(page.getByText("Fixture song lyrics")).toBeVisible();
});

test("live waveform reflects the playing audio samples", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await mockMedia(page, fixture("audio", "wav"));
  await page.route(`${CDN}/asset.wav*`, (route) => serveBytes(route, wav(), "audio/wav"));
  await page.goto("/media/m_fixture_audio");
  await page.getByRole("button", { name: "Play audio" }).click();
  await expect.poll(async () => {
    const path = await page.getByLabel("Live audio waveform").locator("path").last().getAttribute("d");
    return [...(path ?? "").matchAll(/,([\d.-]+)/g)].some((match) => Math.abs(Number(match[1]) - 36) > 1);
  }).toBe(true);
  await page.getByRole("button", { name: "Pause audio" }).click();
  await expect(page.getByRole("button", { name: "Play audio" })).toBeVisible();
});

for (const kind of ["mesh", "splat"] as const) {
  test(`interactive ${kind} supports orbit, zoom, pan, and reset with no WebGL errors`, async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error" && /THREE|WebGL|shader/i.test(message.text())) errors.push(message.text()); });
    const format = kind === "mesh" ? "glb" : "splat";
    await mockMedia(page, fixture(kind, format));
    await page.route(`${CDN}/asset.${format}*`, (route) => route.fulfill({ body: kind === "mesh" ? glb() : splats(), contentType: "application/octet-stream" }));
    await page.goto(`/media/m_fixture_${kind}`);
    await expect(page.getByRole("button", { name: "Reset view" })).toBeVisible();
    const canvas = page.getByLabel(`Interactive 3D ${kind}`);
    await expect(canvas).toBeVisible();
    const box = (await canvas.boundingBox())!;
    const hash = async () => createHash("sha256").update(await canvas.screenshot()).digest("hex");
    let before = await hash();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 130, box.y + box.height / 2 + 40, { steps: 12 });
    await page.mouse.up();
    await expect.poll(hash).not.toBe(before);
    before = await hash();
    await page.mouse.wheel(0, -280);
    await expect.poll(hash).not.toBe(before);
    before = await hash();
    await page.mouse.down({ button: "right" });
    await page.mouse.move(box.x + box.width / 2 + 70, box.y + box.height / 2 - 40, { steps: 10 });
    await page.mouse.up({ button: "right" });
    await expect.poll(hash).not.toBe(before);
    before = await hash();
    await page.getByRole("button", { name: "Reset view" }).click();
    await expect.poll(hash).not.toBe(before);
    before = await hash();
    await page.getByRole("button", { name: "Rotate left", exact: true }).click();
    await page.getByRole("button", { name: "Zoom out", exact: true }).click();
    await expect.poll(hash).not.toBe(before);
    await page.screenshot({ path: test.info().outputPath(`${kind}.png`) });
    expect(errors).toEqual([]);
  });
}

test("missing and private files get useful errors, and retries recover", async ({ page }) => {
  await page.route(`${API}/v1/media_files/file/**`, (route) => route.fulfill({ status: 404, json: { success: false } }));
  await page.goto("/media/m_missing");
  await expect(page.getByRole("heading", { name: "Media not found" })).toBeVisible();
  await page.route(`${API}/v1/media_files/file/**`, (route) => route.fulfill({ status: 403, json: { success: false } }));
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(page.getByRole("heading", { name: "This media is private" })).toBeVisible();
  await mockMedia(page, fixture("image", "png"));
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(page.getByAltText("Fixture image")).toBeVisible();
});

test("a network failure and malformed response do not strand the loading screen", async ({ page }) => {
  await page.route(`${API}/v1/media_files/file/**`, (route) => route.abort());
  await page.goto("/media/m_fixture_image");
  await expect(page.getByRole("heading", { name: "Unable to load this media" })).toBeVisible();
  await page.route(`${API}/v1/media_files/file/**`, (route) => route.fulfill({ json: { success: true } }));
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(page.getByRole("heading", { name: "Unable to load this media" })).toBeVisible();
});

test("broken image offers the original, and unknown formats remain downloadable", async ({ page }) => {
  await mockMedia(page, fixture("image", "png"));
  await page.route(`${CDN}/**`, (route) => route.fulfill({ status: 404 }));
  await page.goto("/media/m_fixture_image");
  await expect(page.getByText("This browser could not display the file.")).toBeVisible();
  await mockMedia(page, fixture("project", "scene_json"));
  await page.goto("/media/m_fixture_project");
  await expect(page.getByText(/A browser preview isn’t available/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Download file" })).toBeVisible();
});

test("download saves the original file, keeping existing CDN query parameters", async ({ page }) => {
  await mockMedia(page, fixture("image", "png"));
  await page.goto("/media/m_fixture_image");
  const request = page.waitForRequest((r) => r.url().startsWith(CDN) && r.url().includes("dl=1"));
  const downloaded = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download file" }).click();
  expect((await downloaded).suggestedFilename()).toBe("m_fixture_image.png");
  expect(new URL((await request).url()).searchParams.get("fixture")).toBe("1");
});

test("mobile layouts and both themes preserve the preview and actions", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await mockMedia(page, fixture("audio", "wav"));
  await page.route(`${CDN}/asset.wav*`, (route) => serveBytes(route, wav(), "audio/wav"));
  await page.goto("/media/m_fixture_audio");
  await expect(page.getByRole("group", { name: "Audio player" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Download file" })).toBeVisible();
  await page.screenshot({ path: test.info().outputPath("mobile-light.png"), fullPage: true });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("body")).toHaveCSS("background-color", "rgb(18, 19, 22)");
  await page.screenshot({ path: test.info().outputPath("mobile-dark.png"), fullPage: true });
});

test("legacy media types, PLY ambiguity, and safe CDN URLs", () => {
  expect(mediaKind({ ...fixture("dimensional", "spz") })).toBe("splat");
  expect(mediaKind({ ...fixture("dimensional", "glb") })).toBe("mesh");
  expect(mediaKind(fixture("mesh", "ply"))).toBe("mesh");
  expect(mediaKind(fixture("splat", "ply"))).toBe("splat");
  expect(mediaKind(fixture("unknown", "mp3"))).toBe("audio");
  expect(mediaKind(fixture("unknown", "webm"))).toBe("video");
  expect(mediaKind({ ...fixture("unknown", "unknown"), media_links: { cdn_url: `${CDN}/IMAGE.PNG?x=.mp4` } })).toBe("image");
  expect(corsMediaUrl(`${CDN}/scene.spz?token=test#preview`)).toBe(`${CDN}/scene.spz?token=test&cors=1#preview`);
  expect(safeMediaUrl("javascript:alert(1)")).toBeUndefined();
});

function fixture(kind: string, format: string): SharedMedia {
  return { token: `m_fixture_${kind}`, media_class: kind, media_type: format,
    maybe_title: `Fixture ${kind}`, created_at: "2026-09-26T00:00:00Z",
    maybe_creator_user: { username: "fixture_artist", display_name: "Fixture Artist" },
    media_links: { cdn_url: `${CDN}/asset.${format}?fixture=1` } };
}

async function mockMedia(page: Page, media: SharedMedia) {
  await page.route(`${API}/v1/media_files/file/**`, (route) => route.fulfill({ json: { success: true, media_file: media } }));
}

function wav() {
  const buffer = Buffer.alloc(44 + 80000);
  buffer.write("RIFF", 0); buffer.writeUInt32LE(buffer.length - 8, 4); buffer.write("WAVEfmt ", 8);
  buffer.writeUInt32LE(16, 16); buffer.writeUInt16LE(1, 20); buffer.writeUInt16LE(1, 22);
  buffer.writeUInt32LE(8000, 24); buffer.writeUInt32LE(16000, 28); buffer.writeUInt16LE(2, 32); buffer.writeUInt16LE(16, 34);
  buffer.write("data", 36); buffer.writeUInt32LE(80000, 40);
  for (let i = 0; i < 40000; i++) buffer.writeInt16LE(Math.round(Math.sin(i * 2 * Math.PI * 440 / 8000) * 12000), 44 + i * 2);
  return buffer;
}

async function serveBytes(route: Route, bytes: Buffer, contentType: string) {
  const range = route.request().headers().range?.match(/bytes=(\d+)-(\d*)/);
  const start = range ? Number(range[1]) : 0;
  const end = range?.[2] ? Math.min(Number(range[2]), bytes.length - 1) : bytes.length - 1;
  await route.fulfill({ status: range ? 206 : 200, contentType,
    headers: { "accept-ranges": "bytes", "access-control-allow-origin": "*",
      ...(range ? { "content-range": `bytes ${start}-${end}/${bytes.length}` } : {}) },
    body: bytes.subarray(start, end + 1) });
}

function glb() {
  const positions = new Float32Array([-1, -1, 0, 1, -1, 0, 0, 1, 0]);
  let json = JSON.stringify({ asset: { version: "2.0" }, scene: 0, scenes: [{ nodes: [0] }], nodes: [{ mesh: 0 }],
    meshes: [{ primitives: [{ attributes: { POSITION: 0 }, material: 0 }] }],
    materials: [{ doubleSided: true, pbrMetallicRoughness: { baseColorFactor: [0.2, 0.5, 1, 1], metallicFactor: 0 } }],
    buffers: [{ byteLength: positions.byteLength }], bufferViews: [{ buffer: 0, byteLength: positions.byteLength }],
    accessors: [{ bufferView: 0, componentType: 5126, count: 3, type: "VEC3", min: [-1, -1, 0], max: [1, 1, 0] }] });
  while (json.length % 4) json += " ";
  const buffer = Buffer.alloc(28 + json.length + positions.byteLength);
  buffer.writeUInt32LE(0x46546c67, 0); buffer.writeUInt32LE(2, 4); buffer.writeUInt32LE(buffer.length, 8);
  buffer.writeUInt32LE(json.length, 12); buffer.writeUInt32LE(0x4e4f534a, 16); buffer.write(json, 20);
  buffer.writeUInt32LE(positions.byteLength, 20 + json.length); buffer.writeUInt32LE(0x004e4942, 24 + json.length);
  Buffer.from(positions.buffer).copy(buffer, 28 + json.length);
  return buffer;
}

function splats() {
  const buffer = Buffer.alloc(32 * 27);
  for (let i = 0; i < 27; i++) {
    const offset = i * 32;
    [i % 3 - 1, Math.floor(i / 3) % 3 - 1, Math.floor(i / 9) - 1, 0.2, 0.2, 0.2].forEach((value, j) => buffer.writeFloatLE(value, offset + j * 4));
    buffer.set([70, 160, 255, 255, 255, 128, 128, 128], offset + 24);
  }
  return buffer;
}
