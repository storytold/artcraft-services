// Model / provider display names and brand marks, ported from the shared
// @storyteller/model-list metadata (ModelMapping, ModelCreatorIconForId).
// Marks are the services SVGs in public/images/services/.

import { mediaUrl } from "./links";

const MODEL_DISPLAY_NAMES: Record<string, string> = {
  // Grok
  grok_image: "Grok Image",
  grok_imagine_image: "Grok Imagine",
  grok_imagine_image_q: "Grok Imagine Quality",
  grok_video: "Grok Video",
  grok_imagine_video: "Grok Imagine",
  grok_imagine_video_1p5: "Grok Imagine 1.5",

  // Flux (Black Forest Labs)
  flux_1_dev: "Flux 1 Dev",
  flux_1_schnell: "Flux 1 Schnell",
  flux_pro_1p1: "Flux Pro 1.1",
  flux_pro_1p1_ultra: "Flux Pro 1.1 Ultra",
  flux_pro_kontext_max: "Flux Pro Kontext Max",
  flux_dev_juggernaut: "Flux Dev Juggernaut",
  flux_pro_1: "Flux Pro (Inpainting)",
  flux_3: "Flux 3",
  flux_3_draft: "Flux 3 Draft",
  flux_pro_1_1: "Flux Pro 1.1",
  flux_pro_1_1_ultra: "Flux Pro 1.1 Ultra",
  flux_2_lora_angles: "Flux 2 LoRA Angles",

  // OpenAI
  gpt_image_1: "GPT Image 1",
  gpt_image_1p5: "GPT Image 1.5",
  gpt_image_2: "GPT Image 2",
  gpt_image_2p5_flare: "GPT Image 2.5 Flare",
  gpt_image_2p5_sunburst: "GPT Image 2.5 Sunburst",
  sora_2: "Sora 2",
  sora_2_pro: "Sora 2 Pro",

  // Kling
  kling_1p6_pro: "Kling 1.6 Pro",
  kling_2p1_pro: "Kling 2.1 Pro",
  kling_2p1_master: "Kling 2.1 Master",
  kling_2p5_turbo_pro: "Kling 2.5 Turbo Pro",
  kling_2p6_pro: "Kling 2.6 Pro",
  kling_3p0_standard: "Kling 3.0 Standard",
  kling_3p0_pro: "Kling 3.0 Pro",
  kling_1_6_pro: "Kling 1.6 Pro",
  kling_2_1_pro: "Kling 2.1 Pro",
  kling_2_1_master: "Kling 2.1 Master",

  // Seedance (ByteDance)
  seedance_1p0_lite: "Seedance 1.0 Lite",
  seedance_1p0_pro: "Seedance 1.0 Pro",
  seedance_1p5_pro: "Seedance 1.5 Pro",
  seedance_2p0: "Seedance 2.0",
  seedance_2p0_fast: "Seedance 2.0 Fast",
  seedance_2p0_mini: "Seedance 2.0 Mini",
  seedance_2p0_bp: "Seedance 2.0 Plus",
  seedance_2p0_bp_fast: "Seedance 2.0 Plus Fast",
  seedance_2p0_bp_mini: "Seedance 2.0 Plus Mini",
  seedance_2p0_bpu: "Seedance 2.0 Plus Ultra",
  seedance_2p0_bpu_fast: "Seedance 2.0 Plus Ultra Fast",
  seedance_2p0_bpu_mini: "Seedance 2.0 Plus Ultra Mini",
  seedance_2p5: "Seedance 2.5",
  seedance_2p5_preview: "Seedance 2.5 Preview",
  seedance_1_0_lite: "Seedance 1.0 Lite",
  preview_model: "Preview Model",
  preview_model_fast: "Preview Model Fast",

  // Seedream / SeedEdit (ByteDance)
  seedream_4: "Seedream 4",
  seedream_4p5: "Seedream 4.5",
  seedream_5_lite: "Seedream 5 Lite",
  seedream_5p0_pro: "Seedream 5.0 Pro",
  seedream_5p0_pro_u: "Seedream 5.0 Pro Ultra",
  seededit_3: "SeedEdit 3",
  seed_audio_1p0: "Seed Audio 1.0",

  // Alibaba
  happy_horse_1p0: "Happy Horse 1.0",
  qwen_edit_2511_angles: "Qwen Edit 2511 Angles",

  // MiniMax (Hailuo)
  minimax_h3: "MiniMax H3",

  // Suno
  suno_music: "Suno Music",
  suno_remix: "Suno Remix",
  suno_sounds: "Suno Sounds",
  suno_sample: "Suno Sample",

  // Google
  veo_2: "Google Veo 2",
  veo_3: "Google Veo 3",
  veo_3_fast: "Google Veo 3 Fast",
  veo_3p1: "Google Veo 3.1",
  veo_3p1_fast: "Google Veo 3.1 Fast",
  veo_3p1_lite: "Google Veo 3.1 Lite",
  gemini_25_flash: "Nano Banana",
  nano_banana: "Nano Banana",
  nano_banana_2: "Nano Banana 2",
  nano_banana_pro: "Nano Banana Pro",

  // Recraft
  recraft_3: "Recraft 3",

  // Vidu
  vidu_q3: "Vidu Q3",
  vidu_q3_turbo: "Vidu Q3 Turbo",

  // Hunyuan (Tencent)
  hunyuan_3d: "Hunyuan 3D",
  hunyuan_3d_2: "Hunyuan 3D 2.0",
  hunyuan_3d_2p0: "Hunyuan 3D 2.0",
  hunyuan_3d_2p1: "Hunyuan 3D 2.1",
  hunyuan_3d_3: "Hunyuan 3D 3.0",
  hunyuan_3d_3_sketch: "Hunyuan 3D 3 Sketch",
  hunyuan_3d_3p1_pro: "Hunyuan 3D 3.1 Pro",
  hunyuan_3d_3p1_rapid: "Hunyuan 3D 3.1 Rapid",
  hunyuan_3d_3p1_part: "Hunyuan 3D 3.1 Part",
  hunyuan_3d_3p1_topology: "Hunyuan 3D 3.1 Smart Topology",
  hunyuan_3d_2_0: "Hunyuan 3D 2.0",
  hunyuan_3d_2_1: "Hunyuan 3D 2.1",

  // Other 3D
  tripo3d_h3p1: "Tripo3D H3.1",
  triposplat: "TripoSplat",
  meshy_v6: "Meshy 6",
  rodin_2p5_fast: "Rodin 2.5 Fast",

  // World Labs
  worldlabs_gaussian: "World Labs Marble",
  marble_0p1_mini: "Marble Mini",
  marble_0p1_plus: "Marble Plus",
  marble_1p0: "Marble 1.0",
  marble_1p0_draft: "Marble 1.0 Draft",
  marble_1p1: "Marble 1.1",
  marble_1p1_plus: "Marble 1.1 Plus",

  // Midjourney
  midjourney: "Midjourney",
  midjourney_v6: "Midjourney V6",
  midjourney_v6p1: "Midjourney V6.1",
  midjourney_v6p1_raw: "Midjourney V6.1 (Raw)",
  midjourney_v7: "Midjourney V7",
  midjourney_v7_raw: "Midjourney V7 (Raw)",
  midjourney_v7_draft: "Midjourney V7 (Draft)",
  midjourney_v7_draft_raw: "Midjourney V7 (Draft Raw)",
  midjourney_7: "Midjourney v7",
  midjourney_7_niji: "Midjourney v7 Niji (Anime)",
  midjourney_8: "Midjourney v8",

  // Beeble
  switch_x: "Beeble SwitchX",
};

// Model-id prefix → services icon file. Order matters: first match wins.
const MODEL_ID_PREFIX_ICONS: [string, string][] = [
  ["flux", "blackforestlabs.svg"],
  ["nano_banana", "google.svg"],
  ["gemini", "google.svg"],
  ["veo", "google.svg"],
  ["gpt_image", "openai.svg"],
  ["sora", "openai.svg"],
  ["midjourney", "midjourney.svg"],
  ["seedream", "bytedance.svg"],
  ["seedance", "bytedance.svg"],
  ["seededit", "bytedance.svg"],
  ["seed_audio", "bytedance.svg"],
  ["kling", "kling.svg"],
  ["grok", "grok.svg"],
  ["vidu", "vidu.svg"],
  ["recraft", "recraft.svg"],
  ["happy_horse", "alibaba.svg"],
  ["wan", "alibaba.svg"],
  ["qwen", "alibaba.svg"],
  ["minimax", "minimax.svg"],
  ["suno", "suno.svg"],
  ["hunyuan", "tencent.svg"],
  ["marble", "worldlabs.svg"],
  ["worldlabs", "worldlabs.svg"],
  ["switch_x", "artcraft.svg"],
];

const PROVIDER_DISPLAY_NAMES: Record<string, string> = {
  artcraft: "ArtCraft",
  fal: "FAL",
  grok: "Grok",
  midjourney: "Midjourney",
  sora: "Sora",
  worldlabs: "World Labs",
};

const PROVIDER_ICONS: Record<string, string> = {
  artcraft: "artcraft.svg",
  fal: "fal.svg",
  grok: "grok.svg",
  midjourney: "midjourney.svg",
  sora: "openai.svg",
  worldlabs: "worldlabs.svg",
};

const serviceIcon = (file: string) => mediaUrl(`/images/services/${file}`);

// Lowercase, dots → underscores, matching the shared lib's normalizer.
const normalizeModelKey = (modelType: string): string =>
  modelType.toLowerCase().replace(/\./g, "_").trim();

export function getModelDisplayName(modelType: string): string {
  return MODEL_DISPLAY_NAMES[normalizeModelKey(modelType)] ?? modelType;
}

export function getModelIcon(modelType: string): string {
  const key = normalizeModelKey(modelType);
  const match = MODEL_ID_PREFIX_ICONS.find(([prefix]) => key.startsWith(prefix));
  return serviceIcon(match?.[1] ?? "generic.svg");
}

export function getProviderDisplayName(provider: string): string {
  return PROVIDER_DISPLAY_NAMES[provider.toLowerCase()] ?? provider;
}

export function getProviderIcon(provider: string): string | null {
  const file = PROVIDER_ICONS[provider.toLowerCase()];
  return file ? serviceIcon(file) : null;
}
