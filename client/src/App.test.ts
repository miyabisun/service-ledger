import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import App from "./App.svelte";
import type { Service } from "./lib/api";
import { rememberListSearch } from "./lib/listReturn";

function json(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), { status });
}

function listed(services: Service[]): Response {
  const counts = { old: 0, changed: 0, unknown: 0 };
  for (const s of services) counts[s.address_status] += 1;
  return json({ services, counts });
}

/** A fake server answering `answer(method, url, body)`; anything else is a failure. */
function serve(
  answer: (method: string, url: string, body: unknown) => Response | undefined,
) {
  const fetchMock = vi.fn<typeof fetch>().mockImplementation((input, init) => {
    const method = init?.method ?? "GET";
    const body = init?.body ? JSON.parse(String(init.body)) : undefined;
    const response = answer(method, String(input), body);
    if (!response) throw new Error(`unexpected ${method} ${String(input)}`);
    return Promise.resolve(response);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

const path = () => window.location.pathname + window.location.search;

const SERVICE: Service = {
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
    rememberListSearch("");
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

  it("opens the address page, marks it changed and returns to the same list", async () => {
    let flow: Service = {
      ...SERVICE,
      id: 7,
      name: "Flow Example Mobile",
      website_url: "https://flow.example.org/",
      address_change_url: "https://flow.example.org/account/address",
      change_method: "member_page",
      address_status: "old",
    };
    const fetchMock = serve((method, url, body) => {
      if (method === "GET" && url === "/api/services?q=flow") {
        return listed([flow]);
      }
      if (method === "GET" && url === "/api/services/7") return json(flow);
      if (method === "PATCH" && url === "/api/services/7") {
        flow = { ...flow, ...(body as Partial<Service>) };
        return json(flow);
      }
    });
    window.history.replaceState(null, "", "/?q=flow");
    render(App);

    const change = await screen.findByRole("link", {
      name: "Flow Example Mobile の住所変更ページ (新しいタブ)",
    });
    expect(change.getAttribute("href")).toBe(
      "https://flow.example.org/account/address",
    );
    expect(change.getAttribute("target")).toBe("_blank");
    expect(change.getAttribute("rel")).toBe("noopener noreferrer");
    await fireEvent.click(
      screen.getByRole("link", { name: /^Flow Example Mobile\s*旧住所/ }),
    );
    expect(path()).toBe("/services/7");
    await screen.findByRole("heading", { name: "Flow Example Mobile" });
    expect(
      screen
        .getByRole("link", { name: "住所変更ページを開く (新しいタブ)" })
        .getAttribute("href"),
    ).toBe("https://flow.example.org/account/address");
    await fireEvent.click(screen.getByRole("radio", { name: "変更済み" }));
    await fireEvent.input(screen.getByLabelText("メモ"), {
      target: { value: "2026-09 に会員ページで変更" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(path()).toBe("/?q=flow"));
    expect((screen.getByRole("searchbox") as HTMLInputElement).value).toBe(
      "flow",
    );
    await waitFor(() =>
      expect(document.querySelector(".card .badge")?.textContent?.trim()).toBe(
        "変更済み",
      ),
    );
    const patch = fetchMock.mock.calls.find(([, init]) => init?.method);
    expect(JSON.parse(String(patch?.[1]?.body))).toMatchObject({
      address_status: "changed",
      note: "2026-09 に会員ページで変更",
    });
  });

  it("refuses a duplicate name with a link that opens the existing record", async () => {
    const target: Service = {
      ...SERVICE,
      id: 9,
      name: "Duplicate Example Power",
    };
    const other: Service = {
      ...SERVICE,
      id: 10,
      name: "Duplicate Example Gas",
    };
    serve((method, url) => {
      if (method === "GET" && url === "/api/services/9") return json(target);
      if (method === "GET" && url === "/api/services/10") return json(other);
      if (method === "PATCH" && url === "/api/services/10") {
        return json(
          {
            error: "同じ名前または同じ Web サイトのサービスが登録済みです",
            code: "duplicate",
            existing_id: 9,
          },
          409,
        );
      }
    });
    window.history.replaceState(null, "", "/services/10");
    render(App);

    await screen.findByRole("heading", { name: "Duplicate Example Gas" });
    const name = screen.getByLabelText("名前") as HTMLInputElement;
    await fireEvent.input(name, {
      target: { value: "duplicate example power" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("登録済みです");
    expect(document.activeElement).toBe(alert);
    expect(name.value).toBe("duplicate example power");

    await fireEvent.click(
      within(alert).getByRole("link", { name: "登録済みのサービスを開く" }),
    );
    expect(path()).toBe("/services/9");
    await screen.findByRole("heading", { name: "Duplicate Example Power" });
  });

  it("asks before deleting; Escape cancels and confirming removes", async () => {
    const doomed: Service = {
      ...SERVICE,
      id: 11,
      name: "Delete Example Water",
    };
    let gone = false;
    const fetchMock = serve((method, url) => {
      if (method === "GET" && url === "/api/services?q=delete+example") {
        return listed(gone ? [] : [doomed]);
      }
      if (method === "GET" && url === "/api/services/11") return json(doomed);
      if (method === "DELETE" && url === "/api/services/11") {
        gone = true;
        return new Response(null, { status: 204 });
      }
    });
    window.history.replaceState(null, "", "/?q=delete%20example");
    render(App);

    await fireEvent.click(
      await screen.findByRole("link", { name: /^Delete Example Water/ }),
    );
    await screen.findByRole("heading", { name: "Delete Example Water" });
    await fireEvent.click(screen.getByRole("button", { name: "削除" }));
    const dialog = screen.getByRole("dialog", { name: "サービスの削除" });
    expect(dialog.textContent).toContain("「Delete Example Water」");
    await waitFor(() =>
      expect(document.activeElement).toBe(
        within(dialog).getByRole("button", { name: "キャンセル" }),
      ),
    );
    await fireEvent.keyDown(document.activeElement as Element, {
      key: "Escape",
    });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(gone).toBe(false);

    await fireEvent.click(screen.getByRole("button", { name: "削除" }));
    await fireEvent.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "削除する",
      }),
    );
    await waitFor(() => expect(path()).toBe("/?q=delete+example"));
    await waitFor(() =>
      expect(screen.getByRole("status").textContent).toBe(
        "一致するサービスがありません",
      ),
    );
    expect(
      fetchMock.mock.calls.filter(([, init]) => init?.method === "DELETE"),
    ).toHaveLength(1);
  });
});
