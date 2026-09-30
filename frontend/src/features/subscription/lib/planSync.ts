import { ApiError } from "../../../shared/lib/apiError";
import { useEntitlementsStore } from "../store/entitlementsStore";

export const PLAN_SYNC_DELAYS_MS = [2000, 5000, 10000];

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

const planNotSyncedYet = (err: unknown) =>
  err instanceof ApiError &&
  err.code === "PRO_REQUIRED" &&
  useEntitlementsStore.getState().tier !== "free";

export async function whilePlanSyncs<T>(action: () => Promise<T>): Promise<T> {
  for (let attempt = 0; ; attempt += 1) {
    try {
      return await action();
    } catch (err) {
      const delay = PLAN_SYNC_DELAYS_MS[attempt];
      if (delay === undefined || !planNotSyncedYet(err)) throw err;
      await wait(delay);
    }
  }
}
