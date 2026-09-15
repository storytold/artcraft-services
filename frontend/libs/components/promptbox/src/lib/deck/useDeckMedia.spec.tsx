import { act, cleanup, fireEvent, render, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "@storyteller/ui-toaster";
import { useDeckMedia } from "./useDeckMedia";

vi.mock("@storyteller/ui-gallery-modal", () => ({ GalleryModal: () => null }));
vi.mock("@storyteller/api", () => ({ downloadFileFromUrl: vi.fn() }));
vi.mock("@storyteller/ui-toaster", () => ({ toast: { error: vi.fn() } }));

describe("QuickTime reference uploads", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: vi.fn(() => "blob:video") });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: vi.fn() });
    vi.spyOn(HTMLMediaElement.prototype, "duration", "get").mockReturnValue(5);
    vi.spyOn(HTMLMediaElement.prototype, "src", "set").mockImplementation(function (this: HTMLMediaElement) {
      queueMicrotask(() => this.dispatchEvent(new Event("loadedmetadata")));
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it.each([false, true])("sets both picker filters when QuickTime is enabled=%s", (enabled) => {
    const { result } = setup(enabled);
    const { container } = render(<>{result.current.fileInputs}</>);
    const videoInputs = Array.from(container.querySelectorAll<HTMLInputElement>("input"))
      .filter((input) => input.accept.includes("video/mp4"));
    expect(videoInputs).toHaveLength(2);
    for (const input of videoInputs) {
      expect(input.accept.includes("video/quicktime")).toBe(enabled);
      expect(input.accept.includes(".mov")).toBe(enabled);
    }
  });

  it("blocks dropped MOV files by default, including when mislabeled as MP4", async () => {
    const { result, uploadVideo } = setup();
    for (const type of ["video/quicktime", "", "video/mp4"]) {
      await act(async () => {
        await result.current.processVideoFiles([new File(["video"], "reference.MOV", { type })]);
      });
    }
    expect(uploadVideo).not.toHaveBeenCalled();
    expect(toast.error).toHaveBeenCalledWith("Please choose an MP4 video.", { id: "video-ref-type" });
  });

  it.each(["video/quicktime", "", "application/octet-stream"])(
    "uploads MOV files with MIME %s when enabled",
    async (type) => {
      const { result, uploadVideo } = setup(true);
      const file = new File(["video"], "reference.MOV", { type });
      await act(async () => {
        await result.current.processVideoFiles([file]);
      });
      expect(uploadVideo).toHaveBeenCalledWith(expect.objectContaining({ assetFile: file }));
    },
  );

  it("routes a MOV without a MIME type through the combined picker", async () => {
    const { result, uploadVideo } = setup(true);
    const { container } = render(<>{result.current.fileInputs}</>);
    const input = container.querySelector<HTMLInputElement>('input[accept^="image/*,video/mp4"]')!;
    const file = new File(["video"], "reference.mov");
    fireEvent.change(input, { target: { files: [file] } });
    await waitFor(() => {
      expect(uploadVideo).toHaveBeenCalledWith(expect.objectContaining({ assetFile: file }));
    });
  });

  it("keeps MP4 uploads available without the feature", async () => {
    const { result, uploadVideo } = setup();
    const file = new File(["video"], "reference.mp4", { type: "video/mp4" });
    await act(async () => {
      await result.current.processVideoFiles([file]);
    });
    expect(uploadVideo).toHaveBeenCalledWith(expect.objectContaining({ assetFile: file }));
  });

  it("still enforces video duration limits for QuickTime", async () => {
    vi.spyOn(HTMLMediaElement.prototype, "duration", "get").mockReturnValue(31);
    const { result, uploadVideo } = setup(true);
    await act(async () => {
      await result.current.processVideoFiles([new File(["video"], "reference.mov", { type: "video/quicktime" })]);
    });
    expect(uploadVideo).not.toHaveBeenCalled();
    expect(toast.error).toHaveBeenCalledWith("Total video duration cannot exceed 30s", { id: "video-ref-limit" });
  });
});

function setup(allowQuicktimeUploads?: boolean) {
  const uploadVideo = vi.fn(async () => {});
  const hook = renderHook(() => useDeckMedia({
    referenceImages: [],
    setReferenceImages: vi.fn(),
    maxImages: 3,
    referenceVideos: [],
    setReferenceVideos: vi.fn(),
    maxVideoTotalSec: 30,
    allowQuicktimeUploads,
    uploadVideo,
  }));
  return { ...hook, uploadVideo };
}
