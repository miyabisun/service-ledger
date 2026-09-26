import { describe, expect, it } from "vitest";

import {
  displayHost,
  isWebUrl,
  parseListParams,
  STATUS_FILTERS,
  toListSearch,
} from "./services";

describe("displayHost", () => {
  it("shows the host without www, port or path", () => {
    expect(displayHost("https://www.Example.com:8443/login?x=1")).toBe(
      "example.com",
    );
    expect(displayHost("http://shop.example.org")).toBe("shop.example.org");
  });

  it("is empty for missing or malformed URLs", () => {
    expect(displayHost(null)).toBe("");
    expect(displayHost("not a url")).toBe("");
  });
});

describe("isWebUrl", () => {
  it("accepts only absolute http(s) URLs for links", () => {
    expect(isWebUrl("https://example.com/address")).toBe(true);
    expect(isWebUrl("HTTP://example.com")).toBe(true);
    expect(isWebUrl("javascript:alert(1)")).toBe(false);
    expect(isWebUrl("/services/1")).toBe(false);
    expect(isWebUrl(null)).toBe(false);
  });
});

describe("list params", () => {
  it("round-trips the query and a known status through the URL", () => {
    const search = toListSearch({ q: "電気 料金", status: "old" });
    expect(search).toBe("?q=%E9%9B%BB%E6%B0%97+%E6%96%99%E9%87%91&status=old");
    expect(parseListParams(search)).toEqual({ q: "電気 料金", status: "old" });
  });

  it("omits defaults and ignores unknown statuses", () => {
    expect(toListSearch({ q: "", status: "" })).toBe("");
    expect(parseListParams("?status=moved&q=")).toEqual({ q: "", status: "" });
  });

  it("offers every status filter with 'all' first", () => {
    expect(STATUS_FILTERS.map((f) => f.value)).toEqual([
      "",
      "old",
      "unknown",
      "changed",
    ]);
  });
});
