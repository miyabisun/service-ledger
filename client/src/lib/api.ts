import type { AddressStatus, ChangeMethod } from "./services";

export interface Service {
  id: number;
  name: string;
  website_url: string | null;
  address_change_url: string | null;
  change_method: ChangeMethod;
  address_status: AddressStatus;
  source: string;
  note: string;
  created_at: string;
  updated_at: string;
}

export type ServiceFields = Omit<Service, "id" | "created_at" | "updated_at">;

export interface ServiceList {
  services: Service[];
  counts: Record<AddressStatus, number>;
}

/** A response the server rejected, with its `{error, code, existing_id}` body. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
    readonly existingId?: number,
  ) {
    super(message);
  }
}

async function request<T>(url: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) {
    let body: { error?: string; code?: string; existing_id?: number } = {};
    try {
      body = await response.json();
    } catch {
      // Non-JSON errors keep the status only.
    }
    throw new ApiError(
      response.status,
      body.code ?? "http",
      body.error ?? `HTTP ${response.status}`,
      body.existing_id,
    );
  }
  return (response.status === 204 ? undefined : await response.json()) as T;
}

export function fetchServices(
  params: { q: string; status: string },
  signal?: AbortSignal,
): Promise<ServiceList> {
  const search = new URLSearchParams();
  if (params.q.trim()) {
    search.set("q", params.q.trim());
  }
  if (params.status) {
    search.set("address_status", params.status);
  }
  const query = search.toString();
  return request(`/api/services${query ? `?${query}` : ""}`, { signal });
}

export function fetchService(
  id: string,
  signal?: AbortSignal,
): Promise<Service> {
  return request(`/api/services/${encodeURIComponent(id)}`, { signal });
}

export function updateService(
  id: number,
  fields: ServiceFields,
): Promise<Service> {
  return request(`/api/services/${id}`, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(fields),
  });
}

export function deleteService(id: number): Promise<void> {
  return request(`/api/services/${id}`, { method: "DELETE" });
}
