import { userAPI } from "../api/user";
import { useAuthStore } from "../store/authStore";
import { ApiError } from "../utils/apiError";

interface LocalActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

const HEARTBEAT_MS = 30_000;

let current: LocalActivity | null = null;
let publishedOwner: string | null = null;
let heartbeat: number | null = null;
let hiddenFor: string | null = null;
let initialized = false;
let chain: Promise<void> = Promise.resolve();

const stopHeartbeat = () => {
  if (heartbeat !== null) {
    window.clearInterval(heartbeat);
    heartbeat = null;
  }
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
  const allowed = auth.user?.showActivity !== false && hiddenFor !== userId;
  if (!current || !allowed) {
    stopHeartbeat();
    if (publishedOwner === userId) {
      publishedOwner = null;
      try {
        await userAPI.clearActivity();
      } catch {}
    }
    return;
  }
  const activity = current;
  try {
    await userAPI.setActivity(activity);
    publishedOwner = userId;
    if (heartbeat === null) {
      heartbeat = window.setInterval(() => {
        const owner = useAuthStore.getState().user?.id ?? null;
        if (!current || owner === null || owner !== publishedOwner) return;
        void userAPI.setActivity(current).catch(() => {});
      }, HEARTBEAT_MS);
    }
  } catch (e) {
    if (e instanceof ApiError && e.code === "ACTIVITY_HIDDEN") {
      hiddenFor = userId;
      publishedOwner = null;
      stopHeartbeat();
    }
  }
};

const sync = () => {
  chain = chain.then(run, run);
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
      state.user?.showActivity !== prev.user?.showActivity;
    if (!idChanged && !visibilityChanged) return;
    if (state.user?.showActivity !== false) hiddenFor = null;
    sync();
  });
};
