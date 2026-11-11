import { describe, expect, it } from "vitest";
import { isOwnedByCurrentAccount } from "./moodboardSync";

describe("isOwnedByCurrentAccount", () => {
  it("lets anyone touch unstamped local boards", () => {
    expect(isOwnedByCurrentAccount({ ownerId: undefined }, null)).toBe(true);
    expect(isOwnedByCurrentAccount({ ownerId: undefined }, "u1")).toBe(true);
  });

  it("restricts owner-stamped boards to that account", () => {
    expect(isOwnedByCurrentAccount({ ownerId: "u1" }, "u1")).toBe(true);
    expect(isOwnedByCurrentAccount({ ownerId: "u1" }, "u2")).toBe(false);
    expect(isOwnedByCurrentAccount({ ownerId: "u1" }, null)).toBe(false);
  });
});
