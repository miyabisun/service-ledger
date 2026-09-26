// The list's last search string, so edit pages return to the same filtered list.
let lastSearch = "";

export function rememberListSearch(search: string): void {
  lastSearch = search;
}

export function listHref(): string {
  return `/${lastSearch}`;
}
