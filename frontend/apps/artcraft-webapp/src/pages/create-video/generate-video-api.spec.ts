import { beforeEach, describe, expect, it, vi } from "vitest";
import { enqueueVideoGeneration } from "./generate-video-api";

const { generateVideo } = vi.hoisted(() => ({ generateVideo: vi.fn() }));

vi.mock("@storyteller/api", () => ({
  OmniGenApi: class {
    generateVideo = generateVideo;
  },
  JobsApi: class {},
  HttpApiError: class extends Error {},
}));

describe("enqueueVideoGeneration output format", () => {
  beforeEach(() => {
    generateVideo.mockReset().mockResolvedValue({
      success: true,
      inference_job_token: "jinf_video",
    });
  });

  it.each(["seedance_2p5", "seedance_2p5_u"])(
    "%s sends the selected output_format",
    async (model) => {
      for (const outputFormat of ["mp4", "mov"]) {
        const result = await enqueueVideoGeneration({
          model,
          prompt: "A running corgi",
          outputFormat,
        });
        expect(result).toEqual({ success: true, jobToken: "jinf_video" });
        expect(generateVideo).toHaveBeenLastCalledWith(expect.objectContaining({
          model,
          output_format: outputFormat,
        }));
      }
    },
  );

  it("sends null when no output format is selected", async () => {
    await enqueueVideoGeneration({ model: "seedance_2p0", prompt: "A running corgi" });
    expect(generateVideo).toHaveBeenCalledWith(expect.objectContaining({
      output_format: null,
    }));
  });
});
