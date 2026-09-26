import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import App from "./App.svelte";

const SERVICE = {
  id: 3,
  name: "Example Water",
  website_url: null,
  address_change_url: null,
  change_method: "bulk",
  address_status: "unknown",
  source: "fixture",
  note: "",
  created_at: "2026-09-26T00:00:00Z",
  updated_at: "2026-09-26T00:00:00Z",
};

describe("App", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    window.history.replaceState(null, "", "/");
  });

  it("keeps the invariant header and restores a deep edit URL", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn<typeof fetch>().mockImplementation((input) => {
        const payload = /^\/api\/services(\?|$)/.test(String(input))
          ? { services: [SERVICE], counts: { old: 0, changed: 0, unknown: 1 } }
          : SERVICE;
        return Promise.resolve(
          new Response(JSON.stringify(payload), { status: 200 }),
        );
      }),
    );
    window.history.replaceState(null, "", "/services/3");

    render(App);

    const header = screen.getByRole("banner");
    const title = header.querySelector('a[href="/"]');
    expect(title?.textContent).toContain("サービス台帳");
    expect(header.querySelectorAll("a, button")).toHaveLength(2);

    await waitFor(() =>
      expect(screen.getByRole("heading", { name: SERVICE.name })).toBeTruthy(),
    );
    const subHeader = document.querySelector(".sub-header");
    expect(subHeader?.querySelectorAll("a, button")).toHaveLength(0);

    await fireEvent.click(title as HTMLElement);
    expect(window.location.pathname).toBe("/");
  });
});
