// Copy and data for the campaign landing pages (Seedance 2.0, Seedance
// 2.5, MiniMax H3, creator spotlight), ported verbatim from the Vite site.
// Approved marketing copy; edit deliberately.

import { mediaUrl } from "./links";

export const HERO_VIMEO_URL =
  "https://player.vimeo.com/video/1169289718?autoplay=1&muted=1&loop=1&background=0&byline=0&portrait=0&title=0";

export type QA = { question: string; answer: string };
export type Tip = { title: string; body: string };

// ── Five reasons (Seedance 2.0 / 2.5) ─────────────────────────────────────

export const REASON_IMAGES = {
  canvas: mediaUrl("/images/2d-3d.png"),
  windows: mediaUrl("/images/windows-logo.png"),
  apple: mediaUrl("/images/apple-logo.png"),
  linux: mediaUrl("/images/linux-logo.png"),
};

// Every model in the studio, for the "Use every model" marquee. Names from
// the shared model list; brand marks are the services SVGs on the live site.
const service = (file: string) => mediaUrl(`/images/services/${file}`);

export type ModelBadge = { name: string; icon: string };

export const MODEL_BADGES: ModelBadge[] = [
  { name: "Seedance 2.0", icon: service("bytedance.svg") },
  { name: "Nano Banana Pro", icon: service("google.svg") },
  { name: "Kling 3.0 Pro", icon: service("kling.svg") },
  { name: "GPT Image 2", icon: service("openai.svg") },
  { name: "Flux Pro 1.1 Ultra", icon: service("blackforestlabs.svg") },
  { name: "Google Veo 3.1", icon: service("google.svg") },
  { name: "Sora 2 Pro", icon: service("openai.svg") },
  { name: "Seedream 4.5", icon: service("bytedance.svg") },
  { name: "Midjourney", icon: service("midjourney.svg") },
  { name: "Grok Video", icon: service("grok.svg") },
  { name: "Marble 0.1 Plus", icon: service("worldlabs.svg") },
  { name: "Kling 2.6 Pro", icon: service("kling.svg") },
  { name: "Nano Banana 2", icon: service("google.svg") },
  { name: "Hunyuan 3D 3.0", icon: service("tencent.svg") },
  { name: "Seedance 1.5 Pro", icon: service("bytedance.svg") },
  { name: "GPT Image 1.5", icon: service("openai.svg") },
  { name: "Flux Pro Kontext Max", icon: service("blackforestlabs.svg") },
  { name: "Google Veo 3 Fast", icon: service("google.svg") },
  { name: "Qwen Edit 2511", icon: service("alibaba.svg") },
  { name: "MiniMax H3", icon: service("minimax.svg") },
  { name: "Kling 2.5 Turbo Pro", icon: service("kling.svg") },
  { name: "Grok Imagine 1.5", icon: service("grok.svg") },
];

// ── Seedance 2.5 ──────────────────────────────────────────────────────────

export const SD25_OVERVIEW: string[] = [
  "Seedance 2.5 is shaping up to be the most anticipated release in AI video. It's the expected next step for ByteDance's Seedance line, the same line whose 2.0 release took the #1 spot on the Artificial Analysis Video Arena in early 2026, leading both text-to-video and image-to-video at once. ByteDance still hasn't put out an official 2.5 spec as of mid-2026, but between the reporting, the leaks, and the obvious 1.0 → 1.5 → 2.0 march, the direction is hard to miss: a real step up, not a minor patch.",
  "What people are hearing is that 2.5 blows past today's ceiling. The chatter points to as much as 4K output, where current generations top out around 720p and 1080p; generation quick enough to feel near-instant; clips that run past the roughly 15-second wall; characters that stay consistent from one session to the next without you re-uploading references every time; and a lot more you can feed in at once: many reference images plus audio and video, all in a single pass.",
  "The part that isn't a rumor is the ground it stands on, and you can put it to work in ArtCraft right now through Seedance 2.0: audio that's generated jointly with the picture and lands in sync, accurate lip-sync across languages, multi-shot stories that keep a character and a style intact through every cut, precise camera control, and a reference mode that anchors a generation to your own images, video, and audio. Every one of those is exactly what 2.5 is expected to push further, and the moment it's out, it'll be in ArtCraft.",
  "One thing worth remembering through all of this: in ArtCraft a video model is never a black box you toss a sentence into and cross your fingers. It's one stage of a pipeline you direct: set up the scene, place your references, frame the camera, then hand the shot off. A better model just makes that last step better. The control is still yours.",
];

export const SD25_DISCLAIMER =
  "To be clear: Seedance 2.5 isn't out. Anything specific on this page (numbers, speeds, lengths) is our read on the rumors, not a promise from ByteDance, and it can change the day it actually ships.";

export const SD25_FAQ: QA[] = [
  {
    question: "Is Seedance 2.5 real, or just hype?",
    answer:
      "Both, a little. The Seedance line is real and very good, so a 2.5 is a safe bet eventually. But as of now there's no official announcement and no published spec. Everything you've read about it, including on this page, is informed speculation. We've flagged it as such rather than dress rumors up as a feature list.",
  },
  {
    question: "When can I actually use it?",
    answer:
      "We don't know, and anyone giving you a hard date is guessing. The moment ByteDance ships it and we can integrate it, it goes into ArtCraft, and we'll update this page. Until then, you're not waiting on us: the current Seedance is already here.",
  },
  {
    question: "What should I do in the meantime?",
    answer:
      "Build the workflow now so you're fast when 2.5 lands. Seedance 2.0 is in ArtCraft today with the features 2.5 is expected to extend: synced audio generated with the video, multi-language lip-sync, multi-shot scenes that hold a character and style across cuts, precise camera control, and reference mode. Direct your shots with those instead of one-off prompts, and the habits transfer straight to the next model.",
  },
  {
    question: "Why use Seedance in ArtCraft instead of a website?",
    answer:
      "Because a browser tab gives you a prompt box and a paywall. ArtCraft gives you the whole stage: compose with images and 3D, lock your characters, frame the camera, then generate. You also keep your work. It's a desktop app you own, not a subscription you rent.",
  },
  {
    question: "Will my older Seedance projects still work when 2.5 arrives?",
    answer:
      "Yes. New models are added alongside the ones you already use. Nothing you've made disappears, and you choose which model runs each generation. When 2.5 is available you can switch to it where it helps and keep everything else exactly as it is.",
  },
];

export const SD25_TIPS: Tip[] = [
  {
    title: "Direct it, don't describe it",
    body: "Treat the prompt like notes to a crew. Call the shot, the move, and the action separately, a short beat at a time, instead of one long wish that the model has to untangle.",
  },
  {
    title: "Make your references do the work",
    body: "Bring in your own images and footage and be explicit about what each is for. A reference you've chosen beats a paragraph of adjectives trying to conjure the same thing.",
  },
  {
    title: "Start from a real frame",
    body: "Compose your opening shot in ArtCraft first (layout, characters, camera) and let the model animate from something solid. Generations grounded in an actual frame drift far less.",
  },
  {
    title: "Keep characters on a leash",
    body: "Give the model clear, consistent looks at your subject from a few angles. The more grounded it is up front, the better your character holds together across a multi-shot sequence.",
  },
];

export const MANIFESTO_WORDS: string[] = [
  "ArtCraft",
  "brings",
  "control",
  "to",
  "AI",
  "image",
  "and",
  "video",
  "generation,",
  "giving",
  "artists",
  "like",
  "you",
  "full",
  "power",
  "over",
  "every",
  "shot.",
];

export const MANIFESTO_EMPHASIS = "control";

// ── MiniMax H3 ────────────────────────────────────────────────────────────

export const H3 = {
  modelId: "minimax_h3",
  name: "MiniMax H3",
  signupSource: "minimax_h3_landing",
  blogUrl: "https://www.minimax.io/blog/minimax-h3",
  icon: service("minimax.svg"),
};

export const H3_SAMPLE_PROMPTS: string[] = [
  "Slow dolly-in on a barista pouring latte art, morning light, soft cafe ambience and the hiss of the steam wand",
  "A humpback whale breaches at golden hour, seabirds calling, waves crashing in stereo, drone shot pulling back",
  'Neon-lit storefront at night, rain on the glass, a flickering sign that reads "OPEN ALL NIGHT", distant traffic hum',
];

export const H3_HIGHLIGHTS: Tip[] = [
  { title: "Native 2K", body: "Sharp 2K output by default, with a faster 768p mode when you want quick drafts." },
  { title: "Up to 15 seconds", body: "Enough room for an actual beat: setup, action, payoff, instead of a four-second loop." },
  { title: "Stereo sound built in", body: "Audio is generated together with the picture, so ambience and effects land in sync." },
  { title: "Truly multimodal", body: "One model that understands text, images, video, and audio as unified input." },
  { title: "Text that reads", body: "Unusually accurate text and logo rendering for signs, packaging, and title cards." },
  { title: "Multi-shot scenes", body: "Holds characters and style across cuts for short sequences, not just single shots." },
];

export const H3_OVERVIEW: string[] = [
  "MiniMax H3 is the newest generation model from MiniMax, the lab behind the Hailuo video family. Rather than a video-only specialist, H3 is a general-purpose multimodal model: it understands text, images, video, and audio as one unified input, and generates video with native stereo sound in a single pass.",
  "In practice, that shows up as a model that follows instructions unusually well. It renders on-screen text and brand marks accurately, transfers motion from reference footage, keeps characters and style consistent across multi-shot sequences, and produces clips up to 15 seconds at 2K by default, with a faster 768p mode for drafts.",
  "MiniMax is positioning H3 for commercial work (advertising, e-commerce, product design, game content) where control and accuracy matter more than lucky rolls. It is also priced aggressively: MiniMax says 2K output costs less than a third of comparable mainstream models.",
  "On ArtCraft you don't have to take anyone's word for it. H3 generations are free right now: type a prompt at the top of this page and judge the model on your own footage.",
];

export type ExampleVideo = {
  label: string;
  src: string;
  poster: string;
  /** Ultra-wide 92:39 source. */
  wide?: boolean;
  spanClass?: string;
};

// Sample generations from MiniMax's announcement, streamed from their CDN;
// only the poster thumbnails are re-hosted on the live site.
export const H3_EXAMPLES: ExampleVideo[] = [
  {
    label: "Native stereo sound, generated with the picture",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-005-1785473681635.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/stereo-sound.jpg"),
    wide: true,
    spanClass: "sm:col-span-2 lg:col-span-6",
  },
  {
    label: "2K performance",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-004-1785473649727.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/2k-performance.jpg"),
    spanClass: "lg:col-span-2",
  },
  {
    label: "Film opening titles",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-006-1785473655612.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/film-opening-titles.jpg"),
    spanClass: "lg:col-span-2",
  },
  {
    label: "Product website",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-007-1785473658537.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/product-website.jpg"),
    spanClass: "lg:col-span-2",
  },
  {
    label: "Animated poster",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-008-1785473649742.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/animated-poster.jpg"),
    spanClass: "lg:col-span-3",
  },
  {
    label: "Advertising and e-commerce",
    src: "https://filecdn.minimax.chat/public/h3-en-v2-video-009-1785473658745.mp4",
    poster: mediaUrl("/videos/minimax-h3/posters/advertising-ecommerce.jpg"),
    spanClass: "sm:col-span-2 lg:col-span-3",
  },
];

export const H3_TIPS: Tip[] = [
  {
    title: "Write the soundtrack too",
    body: "H3 generates audio with the picture, so tell it what you want to hear: room tone, footsteps, a distant siren, a music cue. Prompts that describe sound get noticeably better mixes.",
  },
  {
    title: "Call the shots like a director",
    body: 'Name the shot type, the camera move, and the action as separate short beats. "Slow dolly-in on..." beats one long run-on wish the model has to untangle.',
  },
  {
    title: "Use real text deliberately",
    body: "H3 is unusually good at rendering words. If you want a sign, a label, or a title card in the shot, put the exact text in quotes in your prompt.",
  },
  {
    title: "One scene, one idea",
    body: "Fifteen seconds is a moment, not a movie. Give the clip a single clear action with a beginning and an end, then iterate on the takes you like.",
  },
];

export const H3_FAQ: QA[] = [
  {
    question: "Is MiniMax H3 really free on ArtCraft?",
    answer:
      "Yes. MiniMax H3 generations are currently free for everyone with an ArtCraft account, and creating an account is free too. You can generate right here on this page. If that ever changes, the cost will be shown on the Generate button before you run anything.",
  },
  {
    question: "What is MiniMax H3?",
    answer:
      "H3 is MiniMax's general-purpose multimodal generation model, from the lab behind the Hailuo video family. It takes text, images, video, and audio as unified input and generates video with native stereo sound, with strong instruction following and accurate on-screen text rendering.",
  },
  {
    question: "How long and what resolution are the videos?",
    answer:
      "H3 generates clips up to 15 seconds and is built for 2K output, with a faster 768p mode also available. This page uses the model's default settings to keep things simple; the full ArtCraft app gives you control over resolution, duration, and aspect ratio.",
  },
  {
    question: "Does it generate sound?",
    answer:
      "Yes. H3 generates stereo audio jointly with the video, so ambience, effects, and music land in sync instead of being bolted on afterward. Describe the sound you want in your prompt and it becomes part of the shot.",
  },
  {
    question: "Where do my videos go?",
    answer:
      "Every generation is saved to your ArtCraft library. You can watch and download it right here on this page, or open the ArtCraft app to organize your takes and use them in bigger projects.",
  },
  {
    question: "Can I use my own images or footage?",
    answer:
      "Yes. The full ArtCraft app unlocks the rest of H3: start and end frames, reference images, video, and audio, plus every other leading model in one place.",
  },
];

export const H3_GATE_PERKS: string[] = [
  "MiniMax H3 generations are free right now",
  "Every video is saved to your personal library",
  "One account for the web app and the desktop app",
];

// ── Creator spotlight: jboogxcreative ─────────────────────────────────────

export const JBOOGX = {
  instagramUrl: "https://www.instagram.com/jboogxcreative/",
  siteUrl: "https://www.jboogxcreative.com/",
};

export type InstagramPost = { path: string; title: string; image: string };

// Locally hosted thumbnails (live site, named by shortcode) linking out to
// the reel on Instagram: no embeds, so the page stays fast and on-theme.
const igPost = (path: string, title: string): InstagramPost => ({
  path,
  title,
  image: mediaUrl(`/images/creators/jboogx/${path.split("/")[1]}.jpg`),
});

// Hero stage, in left / center / right order; the center card is the newest.
export const JBOOGX_FEATURED: InstagramPost[] = [
  igPost("reel/DZAO9_LBFw3", "Reel by @jboogxcreative, May 31, 2026"),
  igPost("reel/DXMXQZmBAqW", "Reel by @jboogxcreative, Apr 16, 2026"),
  igPost("reel/Dcvs75lhrn8", "Reel by @jboogxcreative, Sep 1, 2026"),
];

export const JBOOGX_FEED: InstagramPost[] = [
  igPost("reel/Dcn-hmHh3p-", "Reel by @jboogxcreative, Aug 29, 2026"),
  igPost("reel/DcdrZI1hauI", "Reel by @jboogxcreative with @askvenice, Aug 25, 2026"),
  igPost("reel/DclZyHHBZYW", "Reel by @jboogxcreative, Aug 25, 2026"),
  igPost("reel/DcbGmARh2nL", "Reel by @jboogxcreative with @askvenice, Aug 24, 2026"),
];

// Names straight from his Instagram bio, presented as his credits, not ours.
export const JBOOGX_CREDITS: string[] = [
  "Will Smith",
  "Coachella",
  "Wu-Tang Clan",
  "Anyma",
  "Grimes",
];

export const JBOOGX_CRAFT: Tip[] = [
  {
    title: "Long prompts, on purpose",
    body: "These aren't one-liners. A finished prompt reads like a shot list: subject, wardrobe, lighting, lens, camera move. Every beat is written out so the model has nothing left to guess.",
  },
  {
    title: "Seedance 2.5 takes direction",
    body: "All that writing pays off: @ tags for style, precise camera moves, multi-shot sequences that keep characters consistent, and dialogue, music, and effects rendered in sync with the picture.",
  },
  {
    title: "Rewrite, re-roll, refine",
    body: "Generations land in a working library. Tweak a line, re-run the shot, keep what works. Taste is a loop, not a lottery.",
  },
];
