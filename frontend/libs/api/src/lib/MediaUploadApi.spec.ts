// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MediaUploadApi } from "./MediaUploadApi.js";
import { Visibility } from "./enums/Visibility.js";
import { FilterEngineCategories } from "./enums/QueryFilters.js";

const { fetchProxy } = vi.hoisted(() => ({ fetchProxy: vi.fn() }));
vi.mock("@storyteller/tauri-utils", () => ({ FetchProxy: fetchProxy }));

beforeEach(() => {
  fetchProxy.mockReset();
  fetchProxy.mockImplementation(
    async () => new Response(JSON.stringify({ success: true, media_file_token: "m_1" })),
  );
});

function sentForm(): FormData {
  return fetchProxy.mock.calls[0][1].body as FormData;
}

const file = () => new File([new Uint8Array([1, 2, 3])], "a.bin");

describe("MediaUploadApi visibility", () => {
  it.each([
    ["UploadImage", (api: MediaUploadApi) => api.UploadImage({ blob: file(), fileName: "a.png", uuid: "u" })],
    ["UploadAudio", (api: MediaUploadApi) => api.UploadAudio({ blob: file(), fileName: "a.wav", uuid: "u" })],
    ["UploadSpzFile", (api: MediaUploadApi) => api.UploadSpzFile({ file: file(), fileName: "a.spz", uuid: "u" })],
    ["UploadPmx", (api: MediaUploadApi) => api.UploadPmx({ file: file(), fileName: "a.pmx", uuid: "u" })],
    ["UploadNewScene", (api: MediaUploadApi) => api.UploadNewScene({ blob: file(), fileName: "a.scn", uuid: "u" })],
    ["UploadNewVideo", (api: MediaUploadApi) => api.UploadNewVideo({ blob: file(), fileName: "a.mp4", uuid: "u" })],
    [
      "UploadNewEngineAsset",
      (api: MediaUploadApi) =>
        api.UploadNewEngineAsset({
          file: file(),
          fileName: "a.glb",
          uuid: "u",
          engine_category: FilterEngineCategories.OBJECT,
        }),
    ],
  ])("%s leaves visibility to the account preference when none is chosen", async (_name, upload) => {
    await upload(new MediaUploadApi());
    expect(sentForm().has("maybe_visibility")).toBe(false);
  });

  it("sends an explicit choice", async () => {
    await new MediaUploadApi().UploadImage({
      blob: file(),
      fileName: "a.png",
      uuid: "u",
      maybe_visibility: Visibility.Private,
    });
    expect(sentForm().get("maybe_visibility")).toBe("private");
  });
});
