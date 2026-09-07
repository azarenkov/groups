import { userAPI } from "../api/user";
import { useAuthStore } from "../store/authStore";
import { ApiError } from "../utils/apiError";
import { useActivityStore } from "../store/activityStore";

interface LocalActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

const HEARTBEAT_MS = 30_000;
const LEASE_CLEAR_RETRY_MS = 10_000;
const LEASE_CLEAR_MAX_ATTEMPTS = 6;

const sessionId = (() => {
  try {
    return crypto.randomUUID().replace(/-/g, "");
  } catch {
    return `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
  }
})();

let current: LocalActivity | null = null;
let publishedOwner: string | null = null;
let leasedFor: string | null = null;
let leaseClearAttempts = 0;
let heartbeat: number | null = null;
let hiddenFor: string | null = null;
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
  const allowed = auth.user?.showGameActivity !== false && hiddenFor !== userId;
  if (!current || !allowed) {
    stopHeartbeat();
    if (publishedOwner === userId || leasedFor === userId) {
      try {
        await userAPI.clearActivity(sessionId);
        publishedOwner = null;
        dropLease();
        if (useAuthStore.getState().user?.id === userId) {
          useActivityStore.getState().apply(userId, null);
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
      useActivityStore.getState().apply(userId, {
        gameId: stored.gameId,
        name: stored.name,
        startedAt: stored.startedAt,
      });
    }
    startHeartbeat();
  } catch (e) {
    if (e instanceof ApiError && e.code === "ACTIVITY_HIDDEN") {
      hiddenFor = userId;
      publishedOwner = null;
      dropLease();
      stopHeartbeat();
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
