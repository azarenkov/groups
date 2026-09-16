import { isBlockedEitherWay } from "../../store/blockStore";
import { useChannelStore } from "../../store/channelStore";
import { useServerStore } from "../../store/serverStore";
import { useUnreadStore } from "../../store/unreadStore";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";

let activeChannelView: string | null = null;
let activeDmView: string | null = null;

export const getActiveChannelView = () => activeChannelView;
export const getActiveDmView = () => activeDmView;

export const viewIsForeground = () =>
  typeof document === "undefined" ||
  (document.visibilityState === "visible" && document.hasFocus());

export function setActiveChatView(view:
  | { kind: "channel"; id: string }
  | { kind: "dm"; id: string }
  | null) {
  activeChannelView = view?.kind === "channel" ? view.id : null;
  activeDmView = view?.kind === "dm" ? view.id : null;
}

const READ_MARK_THROTTLE_MS = 1500;
const lastChannelReadAt: Record<string, number> = {};
const lastDmReadAt: Record<string, number> = {};

export function forgetReadThrottle(view: { channelId?: string; friendId?: string }) {
  if (view.channelId) lastChannelReadAt[view.channelId] = 0;
  if (view.friendId) lastDmReadAt[view.friendId] = 0;
}

export function throttleMarkChannelRead(channelId: string) {
  const now = Date.now();
  if (now - (lastChannelReadAt[channelId] ?? 0) < READ_MARK_THROTTLE_MS) return;
  lastChannelReadAt[channelId] = now;
  void import("../../api/user").then(({ userAPI }) =>
    userAPI.markChannelRead(channelId).catch(() => {
      lastChannelReadAt[channelId] = 0;
    }),
  );
}

export function throttleMarkDmRead(friendId: string) {
  if (isBlockedEitherWay(friendId)) return;
  const now = Date.now();
  if (now - (lastDmReadAt[friendId] ?? 0) < READ_MARK_THROTTLE_MS) return;
  lastDmReadAt[friendId] = now;
  void import("../../api/user").then(({ userAPI }) =>
    userAPI.markDmRead(friendId).catch(() => {
      lastDmReadAt[friendId] = 0;
    }),
  );
}

export function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

const UNREAD_REFETCH_DELAY_MS = 1000;
let unreadRefetchTimer: ReturnType<typeof setTimeout> | null = null;

export const lastChannelBumpAt: Record<string, number> = {};
export const lastDmBumpAt: Record<string, number> = {};

export function eventTs(v: unknown): number {
  const t = Date.parse(asString(v) ?? "");
  return Number.isFinite(t) ? t : Date.now();
}

export function scheduleUnreadRefetch() {
  if (unreadRefetchTimer) return;
  unreadRefetchTimer = setTimeout(() => {
    unreadRefetchTimer = null;
    void import("../../store/unreadStore").then(({ refetchUnread }) =>
      refetchUnread().catch(() => {}),
    );
  }, UNREAD_REFETCH_DELAY_MS);
}

export function purgeServerLocally(id: string) {
  void import("../../store/rolesStore").then(({ useRolesStore }) => {
    useRolesStore.getState().clearServer(id);
  });
  const serverState = useServerStore.getState();
  const channelState = useChannelStore.getState();
  const voiceState = useVoicePresenceStore.getState();
  const unreadState = useUnreadStore.getState();
  const channelIds = (channelState.channelsByServer[id] ?? []).map((c) => c.id);
  voiceState.purgePendingForServer(id, channelIds);
  for (const cid of channelIds) {
    voiceState.clearChannel(cid);
    unreadState.resetChannel(cid);
  }
  if (
    channelState.selectedChannelId &&
    channelIds.includes(channelState.selectedChannelId)
  ) {
    channelState.setSelectedChannel(null);
  }
  channelState.clearChannels(id);
  if (serverState.selectedServerId === id) {
    serverState.setSelectedServer(null);
  }
  serverState.removeServer(id);
}
