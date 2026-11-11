import { describe, expect, it, vi } from "vitest";

vi.mock("@storyteller/ui-video-editor", () => ({
  CanvasRenderer: class {},
  EditorCore: { getInstance: () => ({}) },
}));

import { coverVisibilityFor } from "./thumbnail-capture";

describe("coverVisibilityFor", () => {
  it("copies the project's visibility", () => {
    expect(coverVisibilityFor("public")).toBe("public");
    expect(coverVisibilityFor("hidden")).toBe("hidden");
    expect(coverVisibilityFor("private")).toBe("private");
  });

  it("falls back to private when the project's visibility is unknown", () => {
    expect(coverVisibilityFor(undefined)).toBe("private");
    expect(coverVisibilityFor("something-new")).toBe("private");
  });
});
