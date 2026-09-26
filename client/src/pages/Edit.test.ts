import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Service } from "../lib/api";
import { rememberListSearch } from "../lib/listReturn";
import { initRouter } from "../lib/router.svelte";
import Edit from "./Edit.svelte";

const SERVICE: Service = {
  id: 5,
  name: "Example Mobile",
  website_url: "https://www.example.com/",
  address_change_url: null,
  change_method: "unknown",
  address_status: "old",
  source: "fictional mail",
  note: "memo",
  created_at: "2026-09-26T00:00:00Z",
  updated_at: "2026-09-26T01:00:00Z",
};

function json(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), { status });
}

describe("Edit", () => {
  let teardown: (() => void) | undefined;

  afterEach(() => {
    cleanup();
    teardown?.();
    vi.unstubAllGlobals();
    rememberListSearch("");
    window.history.replaceState(null, "", "/");
  });

  async function open(fetchMock: ReturnType<typeof vi.fn<typeof fetch>>) {
    vi.stubGlobal("fetch", fetchMock);
    window.history.replaceState(null, "", "/services/5");
    teardown = initRouter();
    render(Edit, { props: { id: "5" } });
    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Example Mobile" }),
      ).toBeTruthy(),
    );
  }

  it("saves the whole form as a PATCH and returns to the filtered list", async () => {
    rememberListSearch("?status=old");
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(SERVICE))
      .mockResolvedValueOnce(json({ ...SERVICE, address_status: "changed" }));
    await open(fetchMock);

    await fireEvent.click(screen.getByRole("radio", { name: "変更済み" }));
    await fireEvent.input(screen.getByLabelText("住所変更ページ URL"), {
      target: { value: "https://www.example.com/address" },
    });
    await fireEvent.input(screen.getByLabelText("Web サイト URL"), {
      target: { value: "" },
    });
    expect(
      screen
        .getByRole("link", { name: "住所変更ページを開く (新しいタブ)" })
        .getAttribute("href"),
    ).toBe("https://www.example.com/address");
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => expect(window.location.pathname).toBe("/"));
    expect(window.location.search).toBe("?status=old");
    const [url, init] = fetchMock.mock.calls[1];
    expect(url).toBe("/api/services/5");
    expect(init?.method).toBe("PATCH");
    expect(JSON.parse(String(init?.body))).toEqual({
      name: "Example Mobile",
      website_url: null,
      address_change_url: "https://www.example.com/address",
      change_method: "unknown",
      address_status: "changed",
      source: "fictional mail",
      note: "memo",
    });
  });

  it("keeps the input and links to the existing record on a duplicate", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(SERVICE))
      .mockResolvedValueOnce(
        json(
          {
            error: "同じ名前または同じ Web サイトのサービスが登録済みです",
            code: "duplicate",
            existing_id: 9,
          },
          409,
        ),
      );
    await open(fetchMock);
    await fireEvent.input(screen.getByLabelText("名前"), {
      target: { value: "Example Power" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "保存" }));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("登録済みです");
    expect(document.activeElement).toBe(alert);
    expect(
      screen
        .getByRole("link", { name: "登録済みのサービスを開く" })
        .getAttribute("href"),
    ).toBe("/services/9");
    expect((screen.getByLabelText("名前") as HTMLInputElement).value).toBe(
      "Example Power",
    );
    expect(window.location.pathname).toBe("/services/5");
  });

  it("deletes only after confirmation", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(json(SERVICE))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    await open(fetchMock);

    await fireEvent.click(screen.getByRole("button", { name: "削除" }));
    const dialog = screen.getByRole("dialog", { name: "サービスの削除" });
    expect(dialog.textContent).toContain("「Example Mobile」");
    expect(fetchMock).toHaveBeenCalledTimes(1);
    await fireEvent.click(screen.getByRole("button", { name: "キャンセル" }));
    expect(screen.queryByRole("dialog")).toBeNull();

    await fireEvent.click(screen.getByRole("button", { name: "削除" }));
    await fireEvent.click(screen.getByRole("button", { name: "削除する" }));
    await waitFor(() => expect(window.location.pathname).toBe("/"));
    expect(fetchMock.mock.calls[1][1]?.method).toBe("DELETE");
  });

  it("explains a missing service with a way back", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn<typeof fetch>()
        .mockResolvedValue(json({ error: "x", code: "not_found" }, 404)),
    );
    render(Edit, { props: { id: "404" } });
    expect(await screen.findByText("サービスが見つかりません")).toBeTruthy();
    expect(
      screen.getByRole("link", { name: "一覧へ戻る" }).getAttribute("href"),
    ).toBe("/");
  });
});
