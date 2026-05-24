import { useAuthStore } from "../store/authStore";

export type WsEventType =
  | "dm_message.sent"
  | "dm_message.edited"
  | "dm_message.deleted"
  | "message.sent"
  | "message.edited"
  | "message.deleted"
  | "channel.created"
  | "channel.renamed"
  | "channel.deleted"
  | "channel.participant.joined"
  | "channel.participant.left"
  | "channel.participant.muted"
  | "channel.participant.unmuted"
  | "channel.participant.connectionAborted"
  | "channel.text.joined"
  | "server.created"
  | "server.deleted"
  | "friendship.requestSent"
  | "friendship.requestAccepted"
  | "friendship.removed"
  | "user.registered"
  | "user.emailVerified"
  | "user.phoneVerified"
  | "user.loggedIn"
  | "user.loggedOut";

export interface WsEnvelope<T = unknown> {
  type: WsEventType;
  data: T;
}

type Listener = (env: WsEnvelope) => void;

const RECONNECT_MIN_MS = 1000;
const RECONNECT_MAX_MS = 30_000;
const HEARTBEAT_MS = 25_000;

function defaultEndpoint(): string {
  const envUrl = process.env.REACT_APP_WS_URL;
  if (envUrl && envUrl.length > 0) return envUrl;
  const base = process.env.REACT_APP_API_URL || "http://localhost:8080/api";
  try {
    const u = new URL(base);
    const proto = u.protocol === "https:" ? "wss:" : "ws:";
    return `${proto}//${u.host}/ws`;
  } catch {
    return "ws://localhost:8080/ws";
  }
}

class WsClient {
  private endpoint: string;
  private socket: WebSocket | null = null;
  private listeners = new Set<Listener>();
  private wantOpen = false;
  private reconnectAttempts = 0;
  private reconnectTimer: number | null = null;
  private heartbeatTimer: number | null = null;
  private refreshInProgress = false;
  private handshakeRefreshTried = false;

  constructor(endpoint: string = defaultEndpoint()) {
    this.endpoint = endpoint;
  }

  connect() {
    this.wantOpen = true;
    this.openSocket();
  }

  disconnect() {
    this.wantOpen = false;
    this.clearTimers();
    if (this.socket) {
      try {
        this.socket.close(1000, "client disconnect");
      } catch {}
      this.socket = null;
    }
    this.reconnectAttempts = 0;
  }

  on(handler: Listener): () => void {
    this.listeners.add(handler);
    return () => {
      this.listeners.delete(handler);
    };
  }

  isOpen(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  private openSocket() {
    const token = useAuthStore.getState().tokens?.accessToken;
    if (!token) {
      this.scheduleReconnect();
      return;
    }
    let socket: WebSocket;
    try {
      socket = new WebSocket(this.endpoint, [`bearer.${token}`]);
    } catch (err) {
      console.warn("[ws] WebSocket construct failed:", err);
      this.scheduleReconnect();
      return;
    }
    this.socket = socket;
    let openedOnce = false;

    socket.onopen = () => {
      openedOnce = true;
      this.reconnectAttempts = 0;
      this.handshakeRefreshTried = false;
      this.startHeartbeat();
    };

    socket.onmessage = (ev) => {
      let env: WsEnvelope;
      try {
        env = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (!env || typeof env !== "object" || !env.type) return;
      for (const l of this.listeners) {
        try {
          l(env);
        } catch (err) {
          console.warn("[ws] listener threw:", err);
        }
      }
    };

    socket.onerror = () => {
    };

    socket.onclose = async (ev) => {
      this.clearTimers();
      this.socket = null;
      if (!this.wantOpen) return;

      const handshakeFailed = !openedOnce;
      const explicitAuthCode = this.isAuthFailure(ev.code);
      if ((handshakeFailed && !this.handshakeRefreshTried) || explicitAuthCode) {
        this.handshakeRefreshTried = true;
        if (await this.tryRefresh()) {
          this.openSocket();
          return;
        }
      }
      this.scheduleReconnect();
    };
  }

  private isAuthFailure(code: number): boolean {
    return code === 1008 || code === 4001 || code === 4003;
  }

  private async tryRefresh(): Promise<boolean> {
    if (this.refreshInProgress) return false;
    this.refreshInProgress = true;
    try {
      const { authAPI } = await import("../api/auth");
      await authAPI.handleRefreshFlow();
      return !!useAuthStore.getState().tokens?.accessToken;
    } catch (err) {
      console.warn("[ws] refresh on auth failure failed:", err);
      return false;
    } finally {
      this.refreshInProgress = false;
    }
  }

  private scheduleReconnect() {
    if (!this.wantOpen) return;
    const attempt = ++this.reconnectAttempts;
    const base = Math.min(
      RECONNECT_MAX_MS,
      RECONNECT_MIN_MS * 2 ** Math.min(attempt, 8),
    );
    const jitter = Math.random() * base * 0.3;
    const delay = base + jitter;
    if (this.reconnectTimer) window.clearTimeout(this.reconnectTimer);
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      if (this.wantOpen) this.openSocket();
    }, delay);
  }

  private startHeartbeat() {
    this.clearHeartbeat();
    this.heartbeatTimer = window.setInterval(() => {
      if (this.socket?.readyState === WebSocket.OPEN) {
        try {
          this.socket.send("ping");
        } catch {}
      }
    }, HEARTBEAT_MS);
  }

  private clearHeartbeat() {
    if (this.heartbeatTimer) {
      window.clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private clearTimers() {
    this.clearHeartbeat();
    if (this.reconnectTimer) {
      window.clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
}

export const ws = new WsClient();
