import { userAPI } from "../api/user";
import { useAuthStore } from "../../auth/store/authStore";
import { ApiError } from "../../../shared/lib/apiError";
import { useActivityStore } from "../store/activityStore";
import { randomUuid } from "../../../shared/lib/ids";

interface LocalActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

const HEARTBEAT_MS = 30_000;
const LEASE_CLEAR_RETRY_MS = 10_000;
const LEASE_CLEAR_MAX_ATTEMPTS = 6;

const sessionId = randomUuid().replace(/-/g, "");

let current: LocalActivity | null = null;
let publishedOwner: string | null = null;
let leasedFor: string | null = null;
let leaseClearAttempts = 0;
let heartbeat: number | null = null;
let hiddenFor: string | null = null;
let appearOffline = false;

export function activityAllowed(params: {
  showGameActivity?: boolean;
  hiddenForUser: boolean;
  appearOffline: boolean;
}): boolean {
  if (params.showGameActivity === false) return false;
  if (params.hiddenForUser) return false;
  return !params.appearOffline;
}
let retryTimer: number | null = null;
let initialized = false;
let chain: Promise<void> = Promise.resolve();

const stopHeartbeat = () => {
  if (heartbeat !== null) {
    window.clearInterval(heartbeat);
    heartbeat = null;
  }
};

const startHeartbeat = () => {
  if (heartbeat !== null) return;
  heartbeat = window.setInterval(() => {
    if (!current) return;
    sync();
  }, HEARTBEAT_MS);
};

const scheduleRetry = (delay = 5000) => {
  if (retryTimer !== null) return;
  retryTimer = window.setTimeout(() => {
    retryTimer = null;
    sync();
  }, delay);
};

const dropLease = () => {
  leasedFor = null;
  leaseClearAttempts = 0;
};

const run = async (): Promise<void> => {
  const auth = useAuthStore.getState();
  const userId = auth.user?.id ?? null;
  if (!userId) {
    stopHeartbeat();
    return;
  }
  if (publishedOwner !== null && publishedOwner !== userId) {
    publishedOwner = null;
    stopHeartbeat();
  }
  if (leasedFor !== null && leasedFor !== userId) {
    dropLease();
    stopHeartbeat();
  }
  const allowed = activityAllowed({
    showGameActivity: auth.user?.showGameActivity,
    hiddenForUser: hiddenFor === userId,
    appearOffline,
  });
  if (!current || !allowed) {
    stopHeartbeat();
    if (publishedOwner === userId || leasedFor === userId) {
      try {
        const cleared = await userAPI.clearActivity(sessionId);
        publishedOwner = null;
        dropLease();
        if (useAuthStore.getState().user?.id === userId) {
          useActivityStore
            .getState()
            .applyEvent(userId, null, cleared.occurredAt ?? null);
        }
      } catch (e) {
        if (e instanceof ApiError && e.code === "ACTIVITY_LEASED") {
          publishedOwner = null;
          leasedFor = userId;
          leaseClearAttempts += 1;
          if (leaseClearAttempts >= LEASE_CLEAR_MAX_ATTEMPTS) dropLease();
          else scheduleRetry(LEASE_CLEAR_RETRY_MS);
        } else {
          scheduleRetry();
        }
      }
    }
    return;
  }
  const activity = current;
  try {
    const stored = await userAPI.setActivity({ ...activity, sessionId });
    publishedOwner = userId;
    dropLease();
    if (useAuthStore.getState().user?.id === userId) {
      useActivityStore.getState().applyEvent(
        userId,
        {
          gameId: stored.gameId,
          name: stored.name,
          startedAt: stored.startedAt,
        },
        stored.changedAt ?? null,
      );
    }
    startHeartbeat();
  } catch (e) {
    if (e instanceof ApiError && e.code === "ACTIVITY_HIDDEN") {
      hiddenFor = userId;
      publishedOwner = null;
      dropLease();
      stopHeartbeat();
      if (useAuthStore.getState().user?.id === userId) {
        useActivityStore.getState().apply(userId, null);
        void userAPI
          .getMe()
          .then((me) => {
            const auth = useAuthStore.getState();
            if (auth.user?.id !== userId || me.id !== userId) return;
            useActivityStore
              .getState()
              .applyEvent(userId, null, me.profileUpdatedAt ?? null);
            auth.setUser({
              ...auth.user,
              bio: me.bio,
              showActivity: me.showActivity,
              showGameActivity: me.showGameActivity,
            });
          })
          .catch(() => {});
      }
    } else if (e instanceof ApiError && e.code === "ACTIVITY_LEASED") {
      publishedOwner = null;
      leasedFor = userId;
      startHeartbeat();
    } else {
      scheduleRetry();
    }
  }
};

function sync(): void {
  chain = chain.then(run, run);
}

export const clearActivityForLogout = (
  accessToken: string | null | undefined,
): Promise<void> => {
  const task = async () => {
    if (publishedOwner === null && leasedFor === null) return;
    stopHeartbeat();
    if (accessToken) {
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          await userAPI.clearActivity(sessionId, accessToken);
          break;
        } catch {
          await new Promise((resolve) => window.setTimeout(resolve, 2000));
        }
      }
    }
    publishedOwner = null;
    dropLease();
  };
  chain = chain.then(task, task);
  return chain;
};

export const setActivityHiddenByPresence = (hidden: boolean): void => {
  if (appearOffline === hidden) return;
  appearOffline = hidden;
  if (hidden) {
    sync();
    return;
  }
  const bridge = window.electron;
  if (!bridge?.getGameActivity) {
    sync();
    return;
  }
  void bridge
    .getGameActivity()
    .then((activity) => {
      current = activity ?? null;
      sync();
    })
    .catch(() => sync());
};

export const initGameActivity = (): void => {
  if (initialized) return;
  const bridge = window.electron;
  if (!bridge?.onGameActivity) return;
  initialized = true;
  bridge.onGameActivity((activity) => {
    current = activity;
    sync();
  });
  void bridge
    .getGameActivity?.()
    .then((activity) => {
      current = activity ?? null;
      sync();
    })
    .catch(() => {});
  useAuthStore.subscribe((state, prev) => {
    const idChanged = state.user?.id !== prev.user?.id;
    const visibilityChanged =
      state.user?.showGameActivity !== prev.user?.showGameActivity;
    if (!idChanged && !visibilityChanged) return;
    if (state.user?.showGameActivity !== false) hiddenFor = null;
    sync();
  });
};
