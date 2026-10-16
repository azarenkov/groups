import { useCallback, useEffect, useRef, useState } from "react";

import { friendshipAPI } from "../api/friendship";
import { useFriendshipStore } from "../store/friendshipStore";
import type { Friendship } from "../../../shared/types/friendship";

export type PendingStatus = "pending_incoming" | "pending_outgoing" | "declined";

export const PENDING_STATUSES: PendingStatus[] = ["pending_incoming", "pending_outgoing", "declined"];

export interface PendingSection {
  rows: Friendship[];
  cursor: string | null;
  loading: boolean;
  error: "first" | "more" | null;
}

const PAGE_SIZE = 50;

const emptySection = (): PendingSection => ({ rows: [], cursor: null, loading: false, error: null });

const emptySections = (): Record<PendingStatus, PendingSection> => ({
  pending_incoming: emptySection(),
  pending_outgoing: emptySection(),
  declined: emptySection(),
});

export function usePendingFriendships(active: boolean) {
  const statusById = useFriendshipStore((s) => s.statusById);
  const [sections, setSections] = useState<Record<PendingStatus, PendingSection>>(emptySections);
  const [busy, setBusy] = useState<ReadonlySet<string>>(() => new Set());
  const [actionFailed, setActionFailed] = useState(false);
  const loadSeq = useRef(0);

  const loadFirst = useCallback(async (status: PendingStatus, seq: number) => {
    setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: true, error: null } }));
    try {
      const page = await friendshipAPI.list({ status, limit: PAGE_SIZE });
      if (seq !== loadSeq.current) return;
      setSections((prev) => ({
        ...prev,
        [status]: { rows: page.friends, cursor: page.nextCursor, loading: false, error: null },
      }));
    } catch {
      if (seq !== loadSeq.current) return;
      setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: false, error: "first" } }));
    }
  }, []);

  const loadMore = useCallback(
    async (status: PendingStatus) => {
      const section = sections[status];
      if (!section.cursor || section.loading) return;
      const seq = loadSeq.current;
      setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: true, error: null } }));
      try {
        const page = await friendshipAPI.list({ status, cursor: section.cursor, limit: PAGE_SIZE });
        if (seq !== loadSeq.current) return;
        setSections((prev) => {
          const known = new Set(prev[status].rows.map((f) => f.friendId));
          return {
            ...prev,
            [status]: {
              rows: [...prev[status].rows, ...page.friends.filter((f) => !known.has(f.friendId))],
              cursor: page.nextCursor,
              loading: false,
              error: null,
            },
          };
        });
      } catch {
        if (seq !== loadSeq.current) return;
        setSections((prev) => ({ ...prev, [status]: { ...prev[status], loading: false, error: "more" } }));
      }
    },
    [sections],
  );

  const retry = (status: PendingStatus) =>
    sections[status].error === "more" ? loadMore(status) : loadFirst(status, loadSeq.current);

  useEffect(() => {
    if (!active) return;
    const seq = ++loadSeq.current;
    void Promise.all(PENDING_STATUSES.map((status) => loadFirst(status, seq)));
  }, [active, statusById, loadFirst]);

  const dropRow = (friendId: string) =>
    setSections((prev) => {
      const next = { ...prev };
      for (const status of PENDING_STATUSES) {
        if (prev[status].rows.some((f) => f.friendId === friendId)) {
          next[status] = { ...prev[status], rows: prev[status].rows.filter((f) => f.friendId !== friendId) };
        }
      }
      return next;
    });

  const runAction = async (friendId: string, action: () => Promise<void>) => {
    setActionFailed(false);
    setBusy((prev) => new Set(prev).add(friendId));
    try {
      await action();
      dropRow(friendId);
    } catch {
      setActionFailed(true);
    } finally {
      setBusy((prev) => {
        const next = new Set(prev);
        next.delete(friendId);
        return next;
      });
    }
  };

  const total = (status: PendingStatus) => Object.values(statusById).filter((s) => s === status).length;
  const countFor = (status: PendingStatus) => Math.max(total(status), sections[status].rows.length);
  const visible = (status: PendingStatus) => countFor(status) > 0;

  return {
    sections,
    busy,
    actionFailed,
    loadMore,
    retry,
    countFor,
    visible,
    accept: (friendId: string) => runAction(friendId, () => useFriendshipStore.getState().accept(friendId)),
    remove: (friendId: string) => runAction(friendId, () => useFriendshipStore.getState().remove(friendId)),
  };
}
