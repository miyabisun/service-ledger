import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Service } from "../lib/api";
import { listHref } from "../lib/listReturn";
import Home from "./Home.svelte";

function service(overrides: Partial<Service>): Service {
  return {
    id: 1,
    name: "Example Mobile",
    website_url: "https://www.example.com/",
    address_change_url: "https://www.example.com/account/address",
    change_method: "member_page",
    address_status: "old",
    source: "fixture",
    note: "",
    created_at: "2026-09-26T00:00:00Z",
    updated_at: "2026-09-26T00:00:00Z",
    ...overrides,
  };
}

const SERVICES = [
  service({}),
  service({
    id: 2,
    name: "Example Power",
    website_url: null,
    address_change_url: "javascript:alert(1)",
    change_method: "phone_or_paper",
    address_status: "changed",
  }),
];

function listResponse(services: Service[]): Response {
  const counts = { old: 0, changed: 0, unknown: 0 };
  for (const s of services) counts[s.address_status] += 1;
  return new Response(JSON.stringify({ services, counts }), { status: 200 });
}

function listContainer(): HTMLElement {
  const container = document.querySelector<HTMLElement>("[data-state]");
  if (!container) throw new Error("list container was not found");
  return container;
}

describe("Home", () => {
  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    vi.useRealTimers();
    window.history.replaceState(null, "", "/");
  });

  it("renders cards with status, method, host and a safe address-change link", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn<typeof fetch>().mockResolvedValue(listResponse(SERVICES)),
    );
    render(Home);
    expect(listContainer().dataset.state).toBe("loading");
    await waitFor(() => expect(listContainer().dataset.state).toBe("success"));

    const edit = screen.getByRole("link", { name: /Example Mobile\s*旧住所/ });
    expect(edit.getAttribute("href")).toBe("/services/1");
    expect(edit.textContent).toContain("会員ページ · example.com");
    const change = screen.getByRole("link", {
      name: "Example Mobile の住所変更ページ (新しいタブ)",
    });
    expect(change.getAttribute("href")).toBe(
      "https://www.example.com/account/address",
    );
    expect(change.getAttribute("target")).toBe("_blank");
    expect(change.getAttribute("rel")).toBe("noopener noreferrer");
    // A stored non-web URL never becomes a link.
    expect(
      screen.queryByRole("link", { name: /Example Power の住所変更/ }),
    ).toBeNull();
    expect(
      screen.getByRole("button", { name: /すべて/ }).textContent,
    ).toContain("2");
    expect(
      screen.getByRole("button", { name: /旧住所/ }).textContent,
    ).toContain("1");
  });

  it("sends search and status to the server and mirrors them in the URL", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockImplementation(() => Promise.resolve(listResponse(SERVICES)));
    vi.stubGlobal("fetch", fetchMock);
    window.history.replaceState(null, "", "/?q=mobile&status=old");
    render(Home);
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(String(fetchMock.mock.calls[0][0])).toBe(
      "/api/services?q=mobile&address_status=old",
    );
    const input = screen.getByRole("searchbox", {
      name: "名前・URL・メモで検索",
    }) as HTMLInputElement;
    expect(input.value).toBe("mobile");
    expect(
      screen
        .getByRole("button", { name: /旧住所/ })
        .getAttribute("aria-pressed"),
    ).toBe("true");

    await fireEvent.click(screen.getByRole("button", { name: /すべて/ }));
    await fireEvent.input(input, { target: { value: "power" } });
    await waitFor(() =>
      expect(String(fetchMock.mock.lastCall?.[0])).toBe(
        "/api/services?q=power",
      ),
    );
    expect(window.location.search).toBe("?q=power");
    expect(listHref()).toBe("/?q=power");
  });

  it("distinguishes an empty ledger from no matches", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn<typeof fetch>()
        .mockImplementation(() => Promise.resolve(listResponse([]))),
    );
    render(Home);
    await waitFor(() => expect(listContainer().dataset.state).toBe("empty"));
    expect(screen.getByText("サービスはまだ登録されていません")).toBeTruthy();

    await fireEvent.input(screen.getByRole("searchbox"), {
      target: { value: "missing" },
    });
    await waitFor(() =>
      expect(screen.getByRole("status").textContent).toBe(
        "一致するサービスがありません",
      ),
    );
  });

  it("shows the error state and recovers through the retry button", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(listResponse(SERVICES));
    vi.stubGlobal("fetch", fetchMock);
    render(Home);
    await waitFor(() => expect(listContainer().dataset.state).toBe("error"));
    await fireEvent.click(screen.getByRole("button", { name: "再試行" }));
    await waitFor(() => expect(listContainer().dataset.state).toBe("success"));
  });

  it("reloads when the tab becomes visible again", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockImplementation(() => Promise.resolve(listResponse(SERVICES)));
    vi.stubGlobal("fetch", fetchMock);
    render(Home);
    await waitFor(() => expect(listContainer().dataset.state).toBe("success"));
    Object.defineProperty(document, "visibilityState", {
      configurable: true,
      value: "visible",
    });
    document.dispatchEvent(new Event("visibilitychange"));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });
});
