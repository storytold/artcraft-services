// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UsersApi } from "./UsersApi";
import { getSignedSession, storeSignedSession } from "./ApiManager";

const { fetchProxy } = vi.hoisted(() => ({ fetchProxy: vi.fn() }));
vi.mock("@storyteller/tauri-utils", () => ({ FetchProxy: fetchProxy }));

// In-memory storage so the test does not depend on which localStorage the
// runtime exposes (newer Node versions shadow jsdom's with their own).
function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => Array.from(items.keys())[index] ?? null,
    removeItem: (key) => {
      items.delete(key);
    },
    setItem: (key, value) => {
      items.set(key, String(value));
    },
  };
}

beforeEach(() => {
  fetchProxy.mockReset();
  vi.stubGlobal("localStorage", memoryStorage());
});

describe("UsersApi.Logout", () => {
  it("sends the stored session with the logout request, then clears it", async () => {
    storeSignedSession("signed-session-value");
    fetchProxy.mockResolvedValue(new Response(JSON.stringify({ success: true })));

    const response = await new UsersApi().Logout();

    expect(response.success).toBe(true);
    const [url, init] = fetchProxy.mock.calls[0];
    expect(new URL(url).pathname).toBe("/v1/logout");
    expect(init.headers.session).toBe("signed-session-value");
    expect(getSignedSession()).toBeNull();
  });

  it("reports failure and still clears the local session when the request fails", async () => {
    storeSignedSession("signed-session-value");
    fetchProxy.mockRejectedValue(new Error("network down"));

    const response = await new UsersApi().Logout();

    expect(response.success).toBe(false);
    expect(response.errorMessage).toBe("network down");
    expect(getSignedSession()).toBeNull();
  });
});
