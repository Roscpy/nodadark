// src/lib/stores/proxyStore.ts
// État global de l'application (Svelte stores)

import { writable, derived } from "svelte/store";

export interface RequestEntry {
  id: string;
  method: string;
  url: string;
  host: string;
  path: string;
  http_version: string;
  request_headers: [string, string][];
  request_body: number[] | null;
  response_status: number | null;
  response_headers: [string, string][];
  response_body: number[] | null;
  duration_ms: number | null;
  timestamp: string;
  state: "pending" | "complete" | "dropped" | "modified" | "error";
  tls: boolean;
  error: string | null;
}

// ── Stores ────────────────────────────────────────────────────

export const proxyRunning  = writable<boolean>(false);
export const proxyPaused   = writable<boolean>(false);
export const proxyPort     = writable<number>(8080);
export const engineConnected = writable<boolean>(false);
export const caPath        = writable<string>("");

export const requests      = writable<Map<string, RequestEntry>>(new Map());
export const requestOrder  = writable<string[]>([]);
export const selectedId    = writable<string | null>(null);

export const filterText    = writable<string>("");
export const statusMessage = writable<string>("");

// ── Derived ──────────────────────────────────────────────────

export const filteredRequests = derived(
  [requests, requestOrder, filterText],
  ([$requests, $order, $filter]) => {
    const f = $filter.toLowerCase().trim();
    return $order
      .map(id => $requests.get(id))
      .filter((r): r is RequestEntry => {
        if (!r) return false;
        if (!f) return true;
        return (
          r.url.toLowerCase().includes(f) ||
          r.host.toLowerCase().includes(f) ||
          r.method.toLowerCase().includes(f) ||
          String(r.response_status ?? "").includes(f)
        );
      });
  }
);

export const selectedRequest = derived(
  [requests, selectedId],
  ([$requests, $id]) => ($id ? $requests.get($id) ?? null : null)
);

// ── Helpers ──────────────────────────────────────────────────

export function upsertRequest(req: RequestEntry) {
  requests.update(map => {
    map.set(req.id, req);
    return map;
  });
  requestOrder.update(order => {
    if (!order.includes(req.id)) {
      const newOrder = [...order, req.id];
      // Limite à 10 000 entrées
      if (newOrder.length > 10_000) newOrder.shift();
      return newOrder;
    }
    return order;
  });
}

export function clearAll() {
  requests.set(new Map());
  requestOrder.set([]);
  selectedId.set(null);
}

export function getStatusClass(status: number | null): string {
  if (!status) return "status-pending";
  if (status >= 500) return "status-5xx";
  if (status >= 400) return "status-4xx";
  if (status >= 300) return "status-3xx";
  return "status-2xx";
}

export function getMethodClass(method: string): string {
  return `method method-${method.toUpperCase()}`;
}

export function bytesToString(bytes: number[] | null): string {
  if (!bytes || bytes.length === 0) return "";
  return new TextDecoder().decode(new Uint8Array(bytes));
}

export function formatSize(bytes: number[] | null): string {
  if (!bytes) return "0B";
  const n = bytes.length;
  if (n < 1024) return `${n}B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)}KB`;
  return `${(n / 1024 / 1024).toFixed(1)}MB`;
}
