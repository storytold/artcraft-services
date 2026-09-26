import { request, type ApiResult } from "./api";
import { safeMediaUrl, type MediaPrompt, type SharedMedia } from "./media";

export async function getSharedMedia(token: string, signal?: AbortSignal): Promise<ApiResult<SharedMedia>> {
  const result = await request<{ media_file?: SharedMedia }>(
    `/v1/media_files/file/${encodeURIComponent(token)}`, { signal },
  );
  if (!result.success) return result;
  const media = result.data.media_file;
  if (!media?.token || !safeMediaUrl(media.media_links?.cdn_url)) {
    return { success: false, errorMessage: "This media file has no available preview." };
  }
  return { success: true, data: media };
}

export async function getMediaPrompt(token: string, signal?: AbortSignal): Promise<ApiResult<MediaPrompt>> {
  const result = await request<{ prompt?: MediaPrompt }>(
    `/v1/prompts/${encodeURIComponent(token)}`, { signal },
  );
  if (!result.success) return result;
  return result.data.prompt
    ? { success: true, data: result.data.prompt }
    : { success: false, errorMessage: "Prompt unavailable" };
}
