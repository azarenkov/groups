import { QueryClient } from "@tanstack/react-query";

import { ApiError } from "../lib/apiError";
import { ws } from "../ws/websocket";

const isTest = process.env.NODE_ENV === "test";

export const QUERY_MAX_RETRIES = 2;

export function shouldRetryQuery(failureCount: number, error: unknown): boolean {
  if (failureCount >= QUERY_MAX_RETRIES) return false;
  const status = error instanceof ApiError ? error.status : undefined;
  if (status !== undefined && status >= 400 && status < 500 && status !== 429) return false;
  return true;
}

export function retryUnless(stop: (error: unknown) => boolean) {
  if (isTest) return false;
  return (failureCount: number, error: unknown) => !stop(error) && shouldRetryQuery(failureCount, error);
}

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30_000,
      gcTime: 10 * 60_000,
      refetchOnWindowFocus: false,
      retry: isTest ? false : shouldRetryQuery,
    },
  },
});

(globalThis as { __riftQueryClient?: QueryClient }).__riftQueryClient = queryClient;

export const queryKeys = {
  pins: (scopeKey: string) => ["pins", scopeKey] as const,
  globalSearch: (kind: string, query: string) => ["globalSearch", kind, query] as const,
  stickerPack: (packId: string) => ["stickerPack", packId] as const,
  invitePreview: (code: string) => ["invitePreview", code] as const,
  privacySettings: () => ["privacySettings"] as const,
  peopleSearch: (term: string) => ["peopleSearch", term] as const,
  searchHistory: (userId: string) => ["searchHistory", userId] as const,
  inbox: (userId: string) => ["inbox", userId] as const,
  inboxUnread: (userId: string) => ["inboxUnread", userId] as const,
  publicProfile: (userId: string) => ["publicProfile", userId] as const,
};

export async function refetchFresh(queryKey: readonly unknown[]): Promise<void> {
  await queryClient.cancelQueries({ queryKey });
  await queryClient.invalidateQueries({ queryKey });
}

export function resetQueryCache(): void {
  queryClient.clear();
}

let installed = false;

export function installQuerySync(): void {
  if (installed) return;
  installed = true;
  ws.onOpen(() => {
    void queryClient.invalidateQueries();
  });
}
