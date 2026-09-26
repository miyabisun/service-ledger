export type AddressStatus = "old" | "changed" | "unknown";
export type ChangeMethod =
  "bulk" | "member_page" | "phone_or_paper" | "unknown";

export interface ListParams {
  q: string;
  status: AddressStatus | "";
}

export const STATUS_LABELS: Record<AddressStatus, string> = {
  old: "旧住所のまま",
  changed: "変更済み",
  unknown: "不明",
};

/** Short badge text; the full label is used in forms. */
export const STATUS_BADGES: Record<AddressStatus, string> = {
  old: "旧住所",
  changed: "変更済み",
  unknown: "不明",
};

export const METHOD_LABELS: Record<ChangeMethod, string> = {
  bulk: "一括手段",
  member_page: "会員ページ",
  phone_or_paper: "電話・書面",
  unknown: "未確認",
};

export const STATUS_FILTERS: { value: AddressStatus | ""; label: string }[] = [
  { value: "", label: "すべて" },
  { value: "old", label: STATUS_BADGES.old },
  { value: "unknown", label: STATUS_BADGES.unknown },
  { value: "changed", label: STATUS_BADGES.changed },
];

function isStatus(value: string | null): value is AddressStatus {
  return value === "old" || value === "changed" || value === "unknown";
}

/** Links to stored URLs are rendered only for absolute http(s) targets. */
export function isWebUrl(url: string | null): url is string {
  return !!url && /^https?:\/\//i.test(url);
}

export function displayHost(url: string | null): string {
  if (!url) {
    return "";
  }
  try {
    return new URL(url).hostname.toLowerCase().replace(/^www\./, "");
  } catch {
    return "";
  }
}

export function toListSearch(params: ListParams): string {
  const search = new URLSearchParams();
  if (params.q) {
    search.set("q", params.q);
  }
  if (params.status) {
    search.set("status", params.status);
  }
  const text = search.toString();
  return text ? `?${text}` : "";
}

export function parseListParams(search: string): ListParams {
  const params = new URLSearchParams(search);
  const status = params.get("status");
  return { q: params.get("q") ?? "", status: isStatus(status) ? status : "" };
}
