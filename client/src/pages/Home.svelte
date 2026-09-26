<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import { fetchServices, type ServiceList } from "../lib/api";
  import { rememberListSearch } from "../lib/listReturn";
  import {
    displayHost,
    isWebUrl,
    METHOD_LABELS,
    parseListParams,
    STATUS_BADGES,
    STATUS_FILTERS,
    toListSearch,
    type AddressStatus,
  } from "../lib/services";

  type ListState = "loading" | "empty" | "error" | "success";

  const initial = parseListParams(window.location.search);
  let query = $state(initial.q);
  let status = $state<AddressStatus | "">(initial.status);
  let result = $state<ServiceList | undefined>();
  let listState = $state<ListState>("loading");

  let total = $derived(
    result ? Object.values(result.counts).reduce((a, b) => a + b, 0) : 0,
  );

  let controller: AbortController | undefined;
  let loadedOnce = false;

  async function load() {
    controller?.abort();
    controller = new AbortController();
    if (!loadedOnce) {
      listState = "loading";
    }
    try {
      const next = await fetchServices({ q: query, status }, controller.signal);
      result = next;
      loadedOnce = true;
      const unfiltered = query.trim() === "" && status === "";
      listState =
        unfiltered && next.services.length === 0 ? "empty" : "success";
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        return;
      }
      listState = "error";
    }
  }

  function countFor(value: AddressStatus | ""): number {
    if (!result) {
      return 0;
    }
    return value === "" ? total : result.counts[value];
  }

  // Keep the URL in step with the filters so reloads and edits return here.
  $effect(() => {
    const search = toListSearch({ q: query.trim(), status });
    rememberListSearch(search);
    if (window.location.pathname === "/") {
      window.history.replaceState(window.history.state, "", `/${search}`);
    }
    const timer = window.setTimeout(() => void load(), loadedOnce ? 150 : 0);
    return () => window.clearTimeout(timer);
  });

  function onVisibilityChange() {
    if (document.visibilityState === "visible") {
      void load();
    }
  }

  $effect(() => {
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      controller?.abort();
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  });
</script>

<section class="content" data-state={listState}>
  <label for="service-search">名前・URL・メモで検索</label>
  <div class="search-field">
    <span class="search-icon"><Icon name="search" /></span>
    <input id="service-search" type="search" bind:value={query} />
  </div>
  <div class="filters" role="group" aria-label="登録住所の状態で絞り込み">
    {#each STATUS_FILTERS as filter (filter.value)}
      <button
        class="chip"
        type="button"
        aria-pressed={status === filter.value}
        onclick={() => (status = filter.value)}
      >
        {filter.label}
        <span class="count">{result ? countFor(filter.value) : ""}</span>
      </button>
    {/each}
  </div>

  {#if listState === "loading"}
    <p class="state">
      <span class="spinner" aria-hidden="true"></span>読み込み中…
    </p>
  {:else if listState === "empty"}
    <div class="state-wrap">
      <p class="state">サービスはまだ登録されていません</p>
      <p class="hint">
        API・MCP・Bitwarden 取り込みで登録すると、ここに並びます。
      </p>
    </div>
  {:else if listState === "error"}
    <div class="state-wrap">
      <p class="state error">読み込みに失敗しました</p>
      <button class="btn" type="button" onclick={() => void load()}>
        再試行
      </button>
    </div>
  {:else if result && result.services.length === 0}
    <p class="state" role="status">一致するサービスがありません</p>
  {:else if result}
    <ul class="cards">
      {#each result.services as service (service.id)}
        {@const host = displayHost(service.website_url)}
        <li class="card">
          <a class="card-main" href={`/services/${service.id}`}>
            <span class="name">{service.name}</span>
            <span class="meta">
              <span class="badge" data-status={service.address_status}>
                {STATUS_BADGES[service.address_status]}
              </span>
              <span class="detail">
                {METHOD_LABELS[service.change_method]}{host ? ` · ${host}` : ""}
              </span>
            </span>
          </a>
          {#if isWebUrl(service.address_change_url)}
            <a
              class="change-link"
              href={service.address_change_url}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={`${service.name} の住所変更ページ (新しいタブ)`}
            >
              住所変更<Icon name="external-link" />
            </a>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style lang="sass">
  label
    display: block
    margin-bottom: var(--sp-2)
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

  .search-field
    position: relative
    margin-bottom: var(--sp-3)

  .search-icon
    position: absolute
    left: var(--sp-2)
    top: 50%
    transform: translateY(-50%)
    color: var(--c-muted)
    pointer-events: none

  input
    width: 100%
    min-width: 0
    padding: var(--sp-2)
    padding-left: calc(var(--sp-2) * 2 + 1.2em)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: var(--c-surface)
    color: var(--c-on-surface)
    font-size: var(--fs-lg)
    line-height: 1.6

    &:focus
      border-color: var(--c-accent)

  .filters
    display: flex
    flex-wrap: wrap
    gap: var(--sp-1)
    margin-bottom: var(--sp-3)

  .chip
    display: inline-flex
    align-items: baseline
    gap: 6px
    min-height: 36px
    padding: 6px var(--sp-2)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: var(--c-surface-raised)
    color: var(--c-on-surface)
    font-size: var(--fs-sm)
    line-height: 1.5
    cursor: pointer

    &:hover
      background: var(--c-hover-1)

    &[aria-pressed="true"]
      border-color: var(--c-accent)
      background: var(--c-accent-subtle)

  .count
    color: var(--c-muted)
    font-size: var(--fs-xs)
    font-variant-numeric: tabular-nums

  .hint
    margin: 0
    color: var(--c-muted)
    font-size: var(--fs-sm)

  .cards
    display: flex
    flex-direction: column
    gap: var(--sp-2)
    margin: 0
    padding: 0
    list-style: none

  .card
    display: flex
    align-items: stretch
    overflow: hidden
    border: 1px solid var(--c-border)
    border-radius: var(--radius-md)
    background: var(--c-surface-raised)

  .card-main
    display: flex
    flex: 1
    flex-direction: column
    gap: 2px
    min-width: 0
    padding: 10px
    color: var(--c-on-surface)
    text-decoration: none

    &:hover
      background: var(--c-hover-1)

  .name
    min-width: 0
    overflow-wrap: anywhere
    font-size: var(--fs-md)
    font-weight: 500
    line-height: 1.3

  .badge
    flex-shrink: 0
    padding: 2px var(--sp-2)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-full)
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

    &[data-status="old"]
      border-color: var(--c-danger)
      background: var(--c-danger-subtle)
      color: var(--c-danger)

  .meta
    display: flex
    align-items: baseline
    gap: var(--sp-2)
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

  .detail
    min-width: 0
    overflow: hidden
    text-overflow: ellipsis
    white-space: nowrap

  .change-link
    display: flex
    flex-shrink: 0
    align-items: center
    gap: var(--sp-1)
    min-height: 44px
    padding: 0 var(--sp-3)
    border-left: 1px solid var(--c-border)
    color: var(--c-link)
    font-size: var(--fs-sm)
    text-decoration: none
    white-space: nowrap

    &:hover
      background: var(--c-hover-1)
      text-decoration: underline
</style>
