import React from "react";

import { useAuthStore } from "../../auth/store/authStore";
import { useBadgeStore } from "../store/badgeStore";
import type { Badge } from "../store/badgeStore";
import { useEntitlementsStore } from "../store/entitlementsStore";

export function useBadge(userId: string | null | undefined): Badge | null {
  const selfId = useAuthStore((s) => s.user?.id);
  const myTier = useEntitlementsStore((s) => s.tier);
  const myStyle = useEntitlementsStore((s) => s.badgeStyle);
  const isSelf = !!userId && userId === selfId;
  const cached = useBadgeStore((s) => (userId && !isSelf ? s.byUser[userId] : undefined));

  React.useEffect(() => {
    if (!userId || isSelf) return;
    useBadgeStore.getState().request(userId);
  }, [userId, isSelf]);

  if (!userId) return null;
  if (isSelf) return myTier === "free" ? null : { tier: myTier, badgeStyle: myStyle };
  return cached ?? null;
}
