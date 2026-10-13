import { useAuthStore } from "../../features/auth/store/authStore";
import { ApiError } from "../lib/apiError";
import { wsEndpoint } from "../config/runtime";
import { Sequencer } from "./sequencer";

const SESSION_DEAD_CODES = new Set([
  "REFRESH_TOKEN_REVOKED",
  "REFRESH_TOKEN_EXPIRED",
  "REFRESH_TOKEN_NOT_FOUND",
  "REFRESH_TOKEN_USER_MISMATCH",
  "SESSION_EXPIRED",
  "INVALID_CREDENTIALS",
  "TOKEN_INVALID_TYPE",
  "TOKEN_DECODING_FAILED",
  "USER_NOT_FOUND",
  "VALIDATION_FAILED",
]);

function isSessionDead(err: unknown): boolean {
  if (!(err instanceof ApiError)) return false;
  if (err.code && SESSION_DEAD_CODES.has(err.code)) return true;
  const s = err.status;
  return s === 400 || s === 401 || s === 403 || s === 422;
}

export type WsEventType =
  | "dm_message.sent"
  | "dm_message.edited"
  | "dm_message.deleted"
  | "dm.read"
  | "channel.read"
  | "message.sent"
  | "message.edited"
  | "message.deleted"
  | "message.hidden"
  | "message.pin_changed"
  | "conversation_pins.updated"
  | "dm_conversation.cleared"
  | "dm_conversation.hidden_changed"
  | "voice.listened"
  | "sequence.skip"
  | "channel.created"
  | "channel.renamed"
  | "channel.deleted"
  | "channel.participant.joined"
  | "channel.participant.left"
  | "channel.participant.muted"
  | "channel.participant.unmuted"
  | "channel.participant.deafened"
  | "channel.participant.undeafened"
  | "channel.participant.connectionAborted"
  | "channel.participant.camera.on"
  | "channel.participant.camera.off"
  | "channel.participant.screen.on"
  | "channel.participant.screen.off"
  | "channel.stream.viewers"
  | "channel.text.joined"
  | "community.created"
  | "community.updated"
  | "community.ownerChanged"
  | "community.deleted"
  | "community.memberJoined"
  | "community.memberLeft"
  | "friendship.requestSent"
  | "friendship.requestAccepted"
  | "friendship.removed"
  | "user.blocked"
  | "user.unblocked"
  | "notification_preference.updated"
  | "community.roles.changed"
  | "user.registered"
  | "user.renamed"
  | "user.profileChanged"
  | "user.entitlementsChanged"
  | "user.activityChanged"
  | "user.emailVerified"
  | "user.phoneVerified"
  | "user.loggedIn"
  | "user.loggedOut"
  | "user.online"
  | "user.offline"
  | "call.incoming"
  | "call.declined"
  | "call.available"
  | "call.ended"
  | "call.participant.joined"
  | "call.participant.left";

export interface WsEnvelope<T = unknown> {
  type: WsEventType;
  data: T;
  partition?: string;
  seq?: number;
  covered?: boolean;
  eventId?: string;
}

type Listener = (env: WsEnvelope) => void;
type Filter = (env: WsEnvelope) => boolean;

const RECONNECT_MIN_MS = 1000;
const RECENT_EVENTS_LIMIT = 1000;
const RECONNECT_MAX_MS = 30_000;
const HEARTBEAT_MS = 25_000;
const REFRESH_MIN_INTERVAL_MS = 60_000;

function defaultEndpoint(): string {
  return wsEndpoint();
}

class WsClient {
  private endpoint: string;
  private socket: WebSocket | null = null;
  private listeners = new Set<Listener>();
  private filters = new Set<Filter>();
  private wantOpen = false;
  private reconnectAttempts = 0;
  private reconnectTimer: number | null = null;
  private heartbeatTimer: number | null = null;
  private refreshInProgress = false;
  private lastRefreshAttemptAt = 0;
  private openListeners = new Set<() => void>();
  private sequencer = new Sequencer((env) => this.deliver(env));
  private recentEvents = new Set<string>();

  constructor(endpoint: string = defaultEndpoint()) {
    this.endpoint = endpoint;
  }

  baseline(partition: string, seq: number): void {
    this.sequencer.baseline(partition, seq);
  }

  overwrittenAfter(partition: string, seq: number): boolean {
    return this.sequencer.overwrittenAfter(partition, seq);
  }

  checkSnapshot(partition: string, seq: number): void {
    this.sequencer.checkSnapshot(partition, seq);
  }

  onResync(listener: (partition: string) => void): () => void {
    return this.sequencer.onResync(listener);
  }

  receive(env: WsEnvelope): void {
    if (this.isRepeat(env)) return;
    this.sequencer.accept(env);
  }

  private isRepeat(env: WsEnvelope): boolean {
    if (typeof env.eventId !== "string") return false;
    const key = `${env.eventId}|${env.type}`;
    if (this.recentEvents.has(key)) return true;
    this.recentEvents.add(key);
    if (this.recentEvents.size > RECENT_EVENTS_LIMIT) {
      const oldest = this.recentEvents.values().next().value;
      if (oldest !== undefined) this.recentEvents.delete(oldest);
    }
    return false;
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
    this.sequencer.reset();
  }

  on(handler: Listener): () => void {
    this.listeners.add(handler);
    return () => {
      this.listeners.delete(handler);
    };
  }

  onOpen(handler: () => void): () => void {
    this.openListeners.add(handler);
    return () => {
      this.openListeners.delete(handler);
    };
  }

  addFilter(filter: Filter): () => void {
    this.filters.add(filter);
    return () => {
      this.filters.delete(filter);
    };
  }

  deliver(env: WsEnvelope) {
    for (const filter of this.filters) {
      if (!filter(env)) return;
    }
    for (const l of this.listeners) {
      try {
        l(env);
      } catch (err) {
        console.warn("[ws] listener threw:", err);
      }
    }
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
      this.startHeartbeat();
      for (const l of this.openListeners) {
        try {
          l();
        } catch (err) {
          console.warn("[ws] open listener threw:", err);
        }
      }
    };

    socket.onmessage = (ev) => {
      let env: WsEnvelope;
      try {
        env = JSON.parse(ev.data);
      } catch {
        return;
      }
      if (!env || typeof env !== "object" || !env.type) return;
      this.receive(env);
    };

    socket.onerror = () => {
    };

    socket.onclose = async (ev) => {
      this.clearTimers();
      this.socket = null;
      if (!this.wantOpen) return;

      const handshakeFailed = !openedOnce;
      const explicitAuthCode = this.isAuthFailure(ev.code);
      const now = Date.now();
      const refreshThrottled =
        now - this.lastRefreshAttemptAt < REFRESH_MIN_INTERVAL_MS;
      const shouldRefresh =
        explicitAuthCode || (handshakeFailed && !refreshThrottled);
      if (shouldRefresh) {
        this.lastRefreshAttemptAt = now;
        if (await this.tryRefresh()) {
          if (!this.wantOpen) return;
          this.openSocket();
          return;
        }
        if (!this.wantOpen) return;
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
      const { authAPI } = await import("../../features/auth/api/auth");
      await authAPI.handleRefreshFlow({ logoutOnFailure: false });
      return !!useAuthStore.getState().tokens?.accessToken;
    } catch (err) {
      if (isSessionDead(err)) {
        console.warn("[ws] refresh definitively failed; signing out:", err);
        this.wantOpen = false;
        useAuthStore.getState().logout();
      } else {
        console.warn("[ws] refresh on auth failure failed (transient):", err);
      }
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
