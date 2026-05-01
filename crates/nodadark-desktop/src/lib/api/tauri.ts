// src/lib/api/tauri.ts
// Wrappers pour appeler les commandes Rust depuis Svelte

import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { upsertRequest, proxyRunning, proxyPaused, proxyPort, engineConnected, caPath, statusMessage } from "../stores/proxyStore";
import type { RequestEntry } from "../stores/proxyStore";

// ── Proxy ────────────────────────────────────────────────────

export async function startProxy(port = 8080): Promise<void> {
  try {
    await invoke("start_proxy", { port });
    proxyRunning.set(true);
    proxyPort.set(port);
    statusMessage.set(`▶ Proxy démarré sur :${port}`);
  } catch (e) {
    statusMessage.set(`✗ Erreur: ${e}`);
  }
}

export async function stopProxy(): Promise<void> {
  try {
    await invoke("stop_proxy");
    proxyRunning.set(false);
    statusMessage.set("■ Proxy arrêté");
  } catch (e) {
    statusMessage.set(`✗ Erreur: ${e}`);
  }
}

export async function togglePause(): Promise<void> {
  try {
    const paused = await invoke<boolean>("toggle_pause");
    proxyPaused.set(paused);
    statusMessage.set(paused ? "⏸ Proxy en pause" : "▶ Proxy repris");
  } catch (e) {
    statusMessage.set(`✗ Erreur: ${e}`);
  }
}

export async function getStatus(): Promise<void> {
  try {
    const s = await invoke<any>("get_status");
    proxyRunning.set(s.running);
    proxyPort.set(s.port);
    proxyPaused.set(s.paused);
    caPath.set(s.ca_path);
    engineConnected.set(s.running);
  } catch (_) {}
}

// ── Requêtes ─────────────────────────────────────────────────

export async function loadRequests(offset = 0, limit = 200, filter?: string): Promise<void> {
  try {
    const result = await invoke<{ items: RequestEntry[]; total: number }>(
      "list_requests", { offset, limit, filter }
    );
    for (const req of result.items) {
      upsertRequest(req);
    }
  } catch (_) {}
}

export async function loadRequest(id: string): Promise<void> {
  try {
    const result = await invoke<{ request: RequestEntry }>("get_request", { id });
    if (result.request) upsertRequest(result.request);
  } catch (_) {}
}

export async function clearRequests(): Promise<void> {
  await invoke("clear_requests");
}

// ── Actions ──────────────────────────────────────────────────

export async function replayRequest(
  id: string,
  modifiedHeaders?: Record<string, string>,
  modifiedBody?: string
): Promise<void> {
  try {
    await invoke("replay_request", {
      id,
      modifiedHeaders: modifiedHeaders ?? {},
      modifiedBody: modifiedBody ?? null,
    });
    statusMessage.set(`↪ Replay envoyé : ${id.slice(0, 8)}…`);
  } catch (e) {
    statusMessage.set(`✗ Replay échoué: ${e}`);
  }
}

export async function dropRequest(id: string): Promise<void> {
  try {
    await invoke("drop_request", { id });
    statusMessage.set(`✂ Requête droppée`);
  } catch (e) {
    statusMessage.set(`✗ Erreur: ${e}`);
  }
}

// ── Événements temps réel ────────────────────────────────────

export async function subscribeToEvents(): Promise<() => void> {
  const unlisten = await listen<any>("engine-event", (event) => {
    const data = event.payload;
    switch (data.type) {
      case "request": {
        upsertRequest({
          id: data.id,
          method: data.method,
          url: data.url,
          host: data.host,
          path: "",
          http_version: "HTTP/1.1",
          request_headers: [],
          request_body: null,
          response_status: null,
          response_headers: [],
          response_body: null,
          duration_ms: null,
          timestamp: data.timestamp,
          state: "pending",
          tls: data.tls,
          error: null,
        });
        break;
      }
      case "response": {
        // Charger le détail complet
        loadRequest(data.id);
        break;
      }
      case "proxy_state": {
        proxyPaused.set(data.paused);
        proxyPort.set(data.port);
        break;
      }
    }
  });
  return unlisten;
}
