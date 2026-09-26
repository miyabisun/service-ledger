import { expect, request, test, type Page } from "@playwright/test";

// Every service here is fictional. EVIDENCE_DIR, when set, keeps viewport screenshots.
const evidence = process.env.EVIDENCE_DIR;
const statuses = ["old", "unknown", "changed"] as const;

async function snap(page: Page, name: string) {
  if (evidence) {
    await page.screenshot({ path: `${evidence}/${name}.png` });
  }
}

async function create(fields: Record<string, unknown>) {
  const api = await request.newContext({ baseURL: "http://127.0.0.1:5187" });
  const response = await api.post("/api/services", {
    data: { source: "e2e fixture", ...fields },
  });
  expect(response.status(), await response.text()).toBe(201);
  const body = await response.json();
  await api.dispose();
  return body as { id: number; name: string };
}

test.beforeAll(async () => {
  // A restarted worker reruns this hook against the same server; seed only once.
  const api = await request.newContext({ baseURL: "http://127.0.0.1:5187" });
  const seeded = await (
    await api.get("/api/services?q=Example%20Service")
  ).json();
  await api.dispose();
  if (seeded.services.length > 0) return;
  for (let i = 1; i <= 100; i += 1) {
    const n = String(i).padStart(3, "0");
    await create({
      name: `Example Service ${n}`,
      website_url: `https://service${n}.example.com/`,
      address_change_url:
        i % 2 ? `https://service${n}.example.com/address` : null,
      address_status: statuses[i % 3],
      change_method: i % 4 ? "member_page" : "phone_or_paper",
    });
  }
  await create({
    name: "Example 長い名前のサービス".repeat(8),
    address_status: "old",
    address_change_url: "https://long.example.net/address",
  });
});

for (const viewport of [
  { width: 1440, height: 900 },
  { width: 375, height: 812 },
  { width: 320, height: 640 },
]) {
  for (const colorScheme of ["dark", "light"] as const) {
    test(`${viewport.width} ${colorScheme}: the list flows with the page`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      await page.emulateMedia({ colorScheme });
      await page.goto("/");
      await expect(page.locator(".card")).toHaveCount(101);
      const geometry = await page.evaluate(() => {
        const cards = [...document.querySelectorAll(".card")].map((card) =>
          card.getBoundingClientRect(),
        );
        const list = document.querySelector(".cards")!;
        return {
          firstTop: cards[0].top,
          visible: cards.filter((c) => c.top >= 48 && c.bottom <= innerHeight)
            .length,
          innerScroll: list.scrollHeight - list.clientHeight,
          documentWidth: document.documentElement.scrollWidth,
          background: getComputedStyle(document.body).backgroundColor,
        };
      });
      test.info().annotations.push({
        type: "geometry",
        description: JSON.stringify(geometry),
      });
      await page.screenshot({
        path: evidence
          ? `${evidence}/list-${viewport.width}-${colorScheme}.png`
          : undefined,
      });
      expect(geometry.documentWidth).toBe(viewport.width);
      expect(geometry.innerScroll).toBeLessThanOrEqual(1);
      // Budgets measured with the header, search field and filters (DESIGN.md).
      const narrow = viewport.width === 320;
      expect(geometry.firstTop).toBeLessThanOrEqual(narrow ? 260 : 210);
      expect(geometry.visible).toBeGreaterThanOrEqual(narrow ? 4 : 8);
      expect(geometry.background).toBe(
        colorScheme === "dark" ? "rgb(25, 25, 25)" : "rgb(250, 246, 239)",
      );
      await page.mouse.wheel(0, 600);
      await expect
        .poll(() => page.evaluate(() => scrollY))
        .toBeGreaterThan(500);
    });
  }
}

test("search and status filter stay in the URL across reloads", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/");
  const search = page.getByRole("searchbox", { name: "名前・URL・メモで検索" });
  await search.fill("service04");
  await expect(page.locator(".card")).toHaveCount(10);
  await page.getByRole("button", { name: /^旧住所/ }).click();
  await expect(page).toHaveURL(/\?q=service04&status=old$/);
  // Services 042, 045 and 048 are the fixtures with the old address.
  await expect(page.locator(".card .name")).toHaveText([
    "Example Service 042",
    "Example Service 045",
    "Example Service 048",
  ]);
  await expect(page.locator('.card .badge[data-status="old"]')).toHaveCount(3);
  await page.reload();
  await expect(search).toHaveValue("service04");
  await expect(page.getByRole("button", { name: /^旧住所/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.locator(".card")).toHaveCount(3);
  await search.fill("no such service");
  await expect(page.getByRole("status")).toHaveText(
    "一致するサービスがありません",
  );
});

test("375px: open the address page, mark changed, return to the same list", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 812 });
  const flow = await create({
    name: "Flow Example Mobile",
    website_url: "https://flow.example.org/",
    address_change_url: "https://flow.example.org/account/address",
    address_status: "old",
  });
  await page.goto("/?q=flow");
  const change = page.getByRole("link", {
    name: "Flow Example Mobile の住所変更ページ (新しいタブ)",
  });
  await expect(change).toHaveAttribute(
    "href",
    "https://flow.example.org/account/address",
  );
  await expect(change).toHaveAttribute("target", "_blank");
  await expect(change).toHaveAttribute("rel", "noopener noreferrer");
  const box = await change.boundingBox();
  expect(box!.height).toBeGreaterThanOrEqual(44);
  await snap(page, "flow-list-before");

  await page
    .getByRole("link", { name: /^Flow Example Mobile\s*旧住所/ })
    .click();
  await expect(page).toHaveURL(new RegExp(`/services/${flow.id}$`));
  await expect(
    page.getByRole("heading", { name: "Flow Example Mobile" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "住所変更ページを開く (新しいタブ)" }),
  ).toHaveAttribute("href", "https://flow.example.org/account/address");
  await snap(page, "flow-edit");
  await page.getByRole("radio", { name: "変更済み" }).check();
  await page.getByLabel("メモ").fill("2026-09 に会員ページで変更");
  await page.getByRole("button", { name: "保存" }).click();

  await expect(page).toHaveURL(/\/\?q=flow$/);
  await expect(page.getByRole("searchbox")).toHaveValue("flow");
  await expect(page.locator(".card .badge")).toHaveText("変更済み");
  await snap(page, "flow-list-after");
});

test("duplicate names are refused with a link to the existing record", async ({
  page,
}) => {
  const target = await create({ name: "Duplicate Example Power" });
  const other = await create({ name: "Duplicate Example Gas" });
  await page.setViewportSize({ width: 320, height: 640 });
  await page.goto(`/services/${other.id}`);
  const name = page.getByLabel("名前");
  await name.fill("duplicate example power");
  await page.getByRole("button", { name: "保存" }).click();
  const alert = page.getByRole("alert");
  await expect(alert).toContainText("登録済みです");
  await expect(alert).toBeFocused();
  await expect(name).toHaveValue("duplicate example power");
  await snap(page, "edit-duplicate-320");
  await alert.getByRole("link", { name: "登録済みのサービスを開く" }).click();
  await expect(page).toHaveURL(new RegExp(`/services/${target.id}$`));
  await expect(
    page.getByRole("heading", { name: "Duplicate Example Power" }),
  ).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
});

test("delete asks first; Escape cancels and confirm removes", async ({
  page,
}) => {
  const doomed = await create({ name: "Delete Example Water" });
  await page.goto("/?q=delete%20example");
  await page.getByRole("link", { name: /^Delete Example Water/ }).click();
  await page.getByRole("button", { name: "削除" }).click();
  const dialog = page.getByRole("dialog", { name: "サービスの削除" });
  await expect(dialog).toContainText("「Delete Example Water」");
  await expect(
    dialog.getByRole("button", { name: "キャンセル" }),
  ).toBeFocused();
  await snap(page, "delete-confirm");
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await page.getByRole("button", { name: "削除" }).click();
  await dialog.getByRole("button", { name: "削除する" }).click();
  await expect(page).toHaveURL(/\/\?q=delete\+example$/);
  await expect(page.getByRole("status")).toHaveText(
    "一致するサービスがありません",
  );
  const response = await page.request.get(`/api/services/${doomed.id}`);
  expect(response.status()).toBe(404);
});

test("keyboard reaches search, filters, cards and the address link with a visible ring", async ({
  page,
}) => {
  await page.goto("/?q=service001");
  await expect(page.locator(".card")).toHaveCount(1);
  await page.getByRole("searchbox").focus();
  for (const expected of ["すべて", "旧住所", "不明", "変更済み"]) {
    await page.keyboard.press("Tab");
    await expect(page.locator(":focus")).toContainText(expected);
  }
  await page.keyboard.press("Tab");
  await expect(page.locator(".card-main")).toBeFocused();
  await expect(page.locator(".card-main")).toHaveCSS("outline-style", "solid");
  await page.keyboard.press("Tab");
  await expect(page.locator(".change-link")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/services\/\d+$/);
});

test("loading, empty, failure and retry at 320px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  let mode: "slow" | "empty" | "error" | "real" = "slow";
  let release!: () => void;
  const pending = new Promise<void>((resolve) => (release = resolve));
  await page.route(/\/api\/services(\?|$)/, async (route) => {
    if (mode === "slow") await pending;
    if (mode === "real") return route.continue();
    await route.fulfill({
      status: mode === "error" ? 500 : 200,
      json:
        mode === "error"
          ? { error: "x", code: "unavailable" }
          : { services: [], counts: { old: 0, changed: 0, unknown: 0 } },
    });
  });
  await page.goto("/");
  await expect(page.getByText("読み込み中…")).toBeVisible();
  mode = "empty";
  release();
  await expect(
    page.getByText("サービスはまだ登録されていません"),
  ).toBeVisible();
  await snap(page, "state-empty-320");
  mode = "error";
  await page.reload();
  await expect(page.getByText("読み込みに失敗しました")).toBeVisible();
  await snap(page, "state-error-320");
  mode = "real";
  await page.getByRole("button", { name: "再試行" }).click();
  await expect(page.locator(".card").first()).toBeVisible();
  const long = page.locator(".card", { hasText: "長い名前" });
  await expect(long).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
  await long.scrollIntoViewIfNeeded();
  await snap(page, "long-name-320");
});

test("theme choice overrides the OS and persists", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto("/");
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("button", { name: "テーマ設定" }).click();
  await page.getByRole("radio", { name: "ライト" }).click();
  await expect(page.locator("body")).toHaveCSS(
    "background-color",
    "rgb(250, 246, 239)",
  );
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("button", { name: "テーマ設定" }).click();
  await page.getByRole("radio", { name: "自動" }).click();
  await expect(page.locator("body")).toHaveCSS(
    "background-color",
    "rgb(25, 25, 25)",
  );
});
