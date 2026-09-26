<script lang="ts">
  import Icon from "../lib/Icon.svelte";
  import Modal from "../lib/Modal.svelte";
  import {
    ApiError,
    deleteService,
    fetchService,
    updateService,
    type Service,
    type ServiceFields,
  } from "../lib/api";
  import { listHref } from "../lib/listReturn";
  import { navigate } from "../lib/router.svelte";
  import {
    isWebUrl,
    METHOD_LABELS,
    STATUS_LABELS,
    type AddressStatus,
    type ChangeMethod,
  } from "../lib/services";

  let { id }: { id: string } = $props();

  type Form = Omit<ServiceFields, "website_url" | "address_change_url"> & {
    website_url: string;
    address_change_url: string;
  };

  let service = $state<Service | undefined>();
  let form = $state<Form | undefined>();
  let pageState = $state<"loading" | "missing" | "error" | "ready">("loading");
  let saving = $state(false);
  let failure = $state<{ message: string; existingId?: number } | undefined>();
  let confirmDelete = $state(false);
  let banner = $state<HTMLElement | undefined>();

  const statuses = Object.entries(STATUS_LABELS) as [AddressStatus, string][];
  const methods = Object.entries(METHOD_LABELS) as [ChangeMethod, string][];

  let controller: AbortController | undefined;

  function localTime(utc: string): string {
    return new Date(utc).toLocaleString("ja-JP", {
      dateStyle: "medium",
      timeStyle: "short",
    });
  }

  async function load(currentId: string) {
    controller?.abort();
    controller = new AbortController();
    pageState = "loading";
    try {
      const loaded = await fetchService(currentId, controller.signal);
      service = loaded;
      form = {
        name: loaded.name,
        website_url: loaded.website_url ?? "",
        address_change_url: loaded.address_change_url ?? "",
        change_method: loaded.change_method,
        address_status: loaded.address_status,
        source: loaded.source,
        note: loaded.note,
      };
      failure = undefined;
      pageState = "ready";
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        return;
      }
      pageState =
        error instanceof ApiError && error.status === 404 ? "missing" : "error";
    }
  }

  $effect(() => {
    void load(id);
    return () => controller?.abort();
  });

  $effect(() => {
    // Move focus to a new failure so keyboard and screen reader users hear it.
    if (failure) {
      banner?.focus();
    }
  });

  function explain(error: unknown): { message: string; existingId?: number } {
    if (error instanceof ApiError) {
      return { message: error.message, existingId: error.existingId };
    }
    return {
      message: "保存できませんでした。通信を確認して再試行してください",
    };
  }

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!service || !form || saving) {
      return;
    }
    saving = true;
    failure = undefined;
    try {
      await updateService(service.id, {
        ...form,
        website_url: form.website_url || null,
        address_change_url: form.address_change_url || null,
      });
      navigate(listHref());
    } catch (error) {
      failure = explain(error);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!service) {
      return;
    }
    saving = true;
    try {
      await deleteService(service.id);
      confirmDelete = false;
      navigate(listHref());
    } catch (error) {
      confirmDelete = false;
      failure = explain(error);
    } finally {
      saving = false;
    }
  }
</script>

<div class="sub-header">
  <h1 class="sub-title">{service ? service.name : "サービスの編集"}</h1>
</div>

<div class="content">
  {#if pageState === "loading"}
    <p class="state">
      <span class="spinner" aria-hidden="true"></span>読み込み中…
    </p>
  {:else if pageState === "missing"}
    <div class="state-wrap">
      <p class="state">サービスが見つかりません</p>
      <a href={listHref()}>一覧へ戻る</a>
    </div>
  {:else if pageState === "error"}
    <div class="state-wrap">
      <p class="state error">読み込みに失敗しました</p>
      <button class="btn" type="button" onclick={() => void load(id)}>
        再試行
      </button>
    </div>
  {:else if service && form}
    <form onsubmit={save}>
      {#if failure}
        <div class="error-banner" role="alert" tabindex="-1" bind:this={banner}>
          {failure.message}
          {#if failure.existingId !== undefined}
            <a href={`/services/${failure.existingId}`}>
              登録済みのサービスを開く
            </a>
          {/if}
        </div>
      {/if}

      <fieldset>
        <legend>登録住所の状態</legend>
        <div class="choices">
          {#each statuses as [value, label] (value)}
            <label class="choice">
              <input
                type="radio"
                name="address_status"
                {value}
                bind:group={form.address_status}
              />
              {label}
            </label>
          {/each}
        </div>
      </fieldset>

      <div class="field">
        <label for="address-change-url">住所変更ページ URL</label>
        <div class="with-link">
          <input
            id="address-change-url"
            type="url"
            inputmode="url"
            placeholder="https://"
            bind:value={form.address_change_url}
          />
          {#if isWebUrl(form.address_change_url)}
            <a
              class="open-link"
              href={form.address_change_url}
              target="_blank"
              rel="noopener noreferrer"
              aria-label="住所変更ページを開く (新しいタブ)"
            >
              開く<Icon name="external-link" />
            </a>
          {/if}
        </div>
      </div>

      <fieldset>
        <legend>変更方法</legend>
        <div class="choices">
          {#each methods as [value, label] (value)}
            <label class="choice">
              <input
                type="radio"
                name="change_method"
                {value}
                bind:group={form.change_method}
              />
              {label}
            </label>
          {/each}
        </div>
      </fieldset>

      <div class="field">
        <label for="name">名前</label>
        <input
          id="name"
          type="text"
          required
          maxlength="200"
          bind:value={form.name}
        />
      </div>

      <div class="field">
        <label for="website-url">Web サイト URL</label>
        <div class="with-link">
          <input
            id="website-url"
            type="url"
            inputmode="url"
            placeholder="https://"
            bind:value={form.website_url}
          />
          {#if isWebUrl(form.website_url)}
            <a
              class="open-link"
              href={form.website_url}
              target="_blank"
              rel="noopener noreferrer"
              aria-label="Web サイトを開く (新しいタブ)"
            >
              開く<Icon name="external-link" />
            </a>
          {/if}
        </div>
      </div>

      <div class="field">
        <label for="source">根拠 (どこで見つけたか)</label>
        <input
          id="source"
          type="text"
          required
          maxlength="1000"
          bind:value={form.source}
        />
      </div>

      <div class="field">
        <label for="note">メモ</label>
        <textarea id="note" rows="4" maxlength="10000" bind:value={form.note}
        ></textarea>
      </div>

      <div class="actions">
        <button class="btn btn-primary" type="submit" disabled={saving}>
          保存
        </button>
        <a class="btn" href={listHref()}>キャンセル</a>
      </div>
      <p class="timestamps">
        登録 {localTime(service.created_at)} ・ 更新 {localTime(
          service.updated_at,
        )}
      </p>

      <div class="danger-zone">
        <button
          class="btn btn-quiet-danger"
          type="button"
          disabled={saving}
          onclick={() => (confirmDelete = true)}
        >
          <Icon name="trash" />削除
        </button>
      </div>
    </form>
  {/if}
</div>

{#if confirmDelete && service}
  <Modal title="サービスの削除" onclose={() => (confirmDelete = false)}>
    <p class="confirm-text">
      「{service.name}」を台帳から削除します。元に戻せません。
    </p>
    <div class="actions">
      <button
        class="btn btn-danger"
        type="button"
        disabled={saving}
        onclick={() => void remove()}
      >
        削除する
      </button>
      <button
        class="btn"
        type="button"
        data-autofocus
        onclick={() => (confirmDelete = false)}
      >
        キャンセル
      </button>
    </div>
  </Modal>
{/if}

<style lang="sass">
  .sub-header
    position: sticky
    top: var(--header-h)
    z-index: 9
    display: flex
    align-items: center
    height: var(--subheader-h)
    padding: 0 var(--sp-3)
    background: var(--c-wash-raised)
    border-bottom: 1px solid var(--c-border)

  .sub-title
    margin: 0
    overflow: hidden
    text-overflow: ellipsis
    white-space: nowrap
    font-size: var(--fs-md)
    font-weight: 500
    line-height: 1.2

  form
    display: flex
    flex-direction: column
    gap: var(--sp-4)

  .error-banner
    margin: 0

    a
      margin-left: var(--sp-2)
      color: inherit

  fieldset
    margin: 0
    padding: 0
    border: none

  legend, label
    display: block
    margin-bottom: var(--sp-2)
    padding: 0
    color: var(--c-muted)
    font-size: var(--fs-xs)
    line-height: 1.4

  .choices
    display: flex
    flex-wrap: wrap
    gap: var(--sp-2)

  .choice
    display: inline-flex
    align-items: center
    gap: var(--sp-2)
    min-height: 44px
    margin: 0
    padding: var(--sp-2) var(--sp-3)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: var(--c-surface-raised)
    color: var(--c-on-surface)
    font-size: var(--fs-md)
    font-weight: 500
    cursor: pointer

    &:hover
      background: var(--c-hover-1)

    &:has(input:checked)
      border-color: var(--c-accent)
      background: var(--c-accent-subtle)

    &:has(input:focus-visible)
      outline: 2px solid var(--c-accent)
      outline-offset: 2px

    input
      margin: 0
      accent-color: var(--c-accent)

  .field
    min-width: 0

  input[type="text"], input[type="url"], textarea
    width: 100%
    min-width: 0
    padding: var(--sp-2)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: var(--c-surface)
    color: var(--c-on-surface)
    font: inherit
    font-size: var(--fs-lg)
    line-height: 1.6

    &:focus
      border-color: var(--c-accent)

  textarea
    resize: vertical

  .with-link
    display: flex
    gap: var(--sp-2)

  .open-link
    display: inline-flex
    flex-shrink: 0
    align-items: center
    gap: var(--sp-1)
    padding: 0 var(--sp-2)
    color: var(--c-link)
    font-size: var(--fs-sm)
    white-space: nowrap

  .actions
    display: flex
    flex-wrap: wrap
    gap: var(--sp-2)

    .btn
      display: inline-flex
      align-items: center
      min-height: 44px
      text-decoration: none

  .timestamps
    margin: 0
    color: var(--c-muted)
    font-size: var(--fs-xs)

  .danger-zone
    padding-top: var(--sp-4)
    border-top: 1px solid var(--c-border)

    .btn
      display: inline-flex
      align-items: center
      gap: var(--sp-1)

  .confirm-text
    margin: 0 0 var(--sp-4)
    overflow-wrap: anywhere
</style>
