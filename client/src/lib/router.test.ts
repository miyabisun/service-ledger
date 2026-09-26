import { waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";

import { initRouter, navigate, router } from "./router.svelte";
import { matchRoute } from "./routes";

describe("matchRoute", () => {
  it("maps / to the home route", () => {
    expect(matchRoute("/")).toEqual({ index: 0, params: {} });
  });

  it("maps /services/:id to the edit route with its id", () => {
    expect(matchRoute("/services/12")).toEqual({
      index: 1,
      params: { id: "12" },
    });
  });

  it("normalizes unknown paths and non-numeric ids to home", () => {
    expect(matchRoute("/no/such/page")).toEqual({ index: 0, params: {} });
    expect(matchRoute("/services/abc")).toEqual({ index: 0, params: {} });
  });
});

describe("router", () => {
  afterEach(() => {
    window.history.replaceState(null, "", "/");
  });

  it("navigate() pushes history and updates the route state", () => {
    const teardown = initRouter();

    navigate("/services/7");

    expect(window.location.pathname).toBe("/services/7");
    expect(router.index).toBe(1);
    expect(router.params.id).toBe("7");
    teardown();
  });

  it("the browser back button returns to the previous route", async () => {
    const teardown = initRouter();
    window.history.replaceState(null, "", "/");
    navigate("/services/7");

    window.history.back();

    await waitFor(() => expect(router.index).toBe(0));
    expect(window.location.pathname).toBe("/");
    teardown();
  });

  it("intercepts clicks on internal links", () => {
    const teardown = initRouter();
    const anchor = document.createElement("a");
    anchor.href = "/services/12";
    anchor.textContent = "card";
    document.body.appendChild(anchor);

    anchor.click();

    expect(window.location.pathname).toBe("/services/12");
    expect(router.index).toBe(1);
    anchor.remove();
    teardown();
  });
});
