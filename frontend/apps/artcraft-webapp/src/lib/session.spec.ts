import { beforeEach, describe, expect, it, vi } from "vitest";
import { onAccountChange, reconcileAccountOwner } from "./session";

vi.mock("@storyteller/api", () => ({ UsersApi: class {} }));

describe("reconcileAccountOwner", () => {
  beforeEach(() => localStorage.clear());

  it("clears account caches only when the identity changes", () => {
    const clear = vi.fn();
    onAccountChange(clear);

    reconcileAccountOwner("user_a"); // first record: nothing to discard
    reconcileAccountOwner("user_a");
    expect(clear).not.toHaveBeenCalled();

    reconcileAccountOwner(undefined); // logout
    expect(clear).toHaveBeenCalledTimes(1);

    reconcileAccountOwner(undefined);
    expect(clear).toHaveBeenCalledTimes(1);

    reconcileAccountOwner("user_b"); // someone else logs in
    expect(clear).toHaveBeenCalledTimes(2);
  });
});
