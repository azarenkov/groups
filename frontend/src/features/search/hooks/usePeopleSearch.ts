import React from "react";
import { useQuery } from "@tanstack/react-query";

import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import type { FriendshipStatus } from "../../../shared/types/friendship";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useBlockStore } from "../../social/store/blockStore";
import { peopleAPI, type PersonHit, type PersonRelation } from "../api/people";

export const PEOPLE_SEARCH_MIN_CHARS = 2;
export const PEOPLE_SEARCH_MAX_CHARS = 32;
export const PEOPLE_SEARCH_DEBOUNCE_MS = 300;
export const PEOPLE_SEARCH_LIMIT = 8;

export const normalizePeopleQuery = (raw: string): string | null => {
  const trimmed = raw.trim();
  const length = Array.from(trimmed).length;
  return length >= PEOPLE_SEARCH_MIN_CHARS && length <= PEOPLE_SEARCH_MAX_CHARS ? trimmed.toLowerCase() : null;
};

const RELATION_BY_STATUS: Partial<Record<FriendshipStatus, PersonRelation>> = {
  accepted: "friend",
  pending_outgoing: "outgoing",
  pending_incoming: "incoming",
};

export const liveRelation = (
  hit: PersonHit,
  friendships: { loaded: boolean; statusById: Record<string, FriendshipStatus> },
): PersonRelation => {
  if (!friendships.loaded) return hit.relation;
  const status = friendships.statusById[hit.id];
  return status ? RELATION_BY_STATUS[status] ?? "none" : "none";
};

export function usePeopleSearch(raw: string) {
  const normalized = normalizePeopleQuery(raw);
  const [searched, setSearched] = React.useState<string | null>(normalized);
  React.useEffect(() => {
    if (normalized === null) {
      setSearched(null);
      return;
    }
    const timer = window.setTimeout(() => setSearched(normalized), PEOPLE_SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [normalized]);
  const query = useQuery(
    {
      queryKey: queryKeys.peopleSearch(searched ?? ""),
      queryFn: () => peopleAPI.search(searched as string, PEOPLE_SEARCH_LIMIT),
      enabled: searched !== null,
      staleTime: 15_000,
    },
    queryClient,
  );
  const statusById = useFriendshipStore((s) => s.statusById);
  const loaded = useFriendshipStore((s) => s.loaded);
  const blocked = useBlockStore((s) => s.blocked);
  const blockedBy = useBlockStore((s) => s.blockedBy);
  const seenBlocks = React.useRef({ blocked, blockedBy });
  React.useEffect(() => {
    if (seenBlocks.current.blocked === blocked && seenBlocks.current.blockedBy === blockedBy) return;
    seenBlocks.current = { blocked, blockedBy };
    void queryClient.invalidateQueries({ queryKey: ["peopleSearch"] });
  }, [blocked, blockedBy]);
  const current = searched !== null && searched === normalized;
  const people = React.useMemo(
    () =>
      (current ? query.data ?? [] : [])
        .filter((hit) => !blocked[hit.id] && !blockedBy[hit.id])
        .map((hit) => ({
          ...hit,
          relation: liveRelation(hit, { loaded, statusById }),
        })),
    [current, query.data, loaded, statusById, blocked, blockedBy],
  );
  return {
    people,
    loading: normalized !== null && (!current || (query.isFetching && !query.data)),
    failed: query.isError,
  };
}
