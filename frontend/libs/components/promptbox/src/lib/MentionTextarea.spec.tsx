import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@storyteller/icons", () => ({ DynamicIcon: () => null }));
vi.mock("./deck/DeckCard", () => ({ DeckPreviewModal: () => null }));
vi.mock("./MentionChipMenu", () => ({ MentionChipMenu: () => null }));

import { MentionTextarea } from "./MentionTextarea";

afterEach(cleanup);

describe("MentionTextarea colorMap", () => {
  it("does not turn a hostile color into markup", () => {
    const payload = '"><img src=x onerror="window.__pwned=1">';
    const { container } = render(
      <MentionTextarea
        value="hi @Video1 and @Image1"
        onChange={() => undefined}
        mentionItems={[
          { label: "@Video1", type: "video" },
          { label: "@Image1", type: "image" },
        ]}
        colorMap={{ "@Video1": payload, "@Image1": "rgb(1, 2, 3)" }}
      />,
    );
    expect(container.querySelector("img")).toBeNull();
    expect(container.innerHTML).not.toContain("onerror");
    const colored = Array.from(container.querySelectorAll<HTMLElement>("[style]"))
      .map((el) => el.style.color)
      .filter(Boolean);
    expect(colored).toContain("rgb(1, 2, 3)");
  });
});
