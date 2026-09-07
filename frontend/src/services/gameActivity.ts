import { userAPI } from "../api/user";
import { useAuthStore } from "../store/authStore";
import { ApiError } from "../utils/apiError";

interface LocalActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

const HEARTBEAT_MS = 30_000;

const sessionId = (() => {
  try {
    return crypto.randomUUID().replace(/-/g, "");
  } catch {
    return `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
  }
})();

let current: LocalActivity | null = null;
let publishedOwner: string | null = null;
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

const scheduleRetry = () => {
  if (retryTimer !== null) return;
  retryTimer = window.setTimeout(() => {
    retryTimer = null;
    sync();
  }, 5000);
};

const run = async (): Promise<void> => {
  const auth = useAuthStore.getState();
  const userId = auth.user?.id ?? null;
  if (publishedOwner !== null && publishedOwner !== userId) {
    publishedOwner = null;
    stopHeartbeat();
  }
  if (!userId) {
    stopHeartbeat();
    return;
  }
  const allowed = auth.user?.showGameActivity !== false && hiddenFor !== userId;
  if (!current || !allowed) {
    stopHeartbeat();
    if (publishedOwner === userId) {
      try {
        await userAPI.clearActivity(sessionId);
        publishedOwner = null;
      } catch {
        scheduleRetry();
      }
    }
    return;
  }
  const activity = current;
  try {
    await userAPI.setActivity({ ...activity, sessionId });
    publishedOwner = userId;
    if (heartbeat === null) {
      heartbeat = window.setInterval(() => {
        if (!current) return;
        sync();
      }, HEARTBEAT_MS);
    }
  } catch (e) {
    if (e instanceof ApiError && e.code === "ACTIVITY_HIDDEN") {
      hiddenFor = userId;
      publishedOwner = null;
      stopHeartbeat();
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
    if (publishedOwner === null) return;
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
