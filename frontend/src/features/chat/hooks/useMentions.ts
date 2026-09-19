import React from "react";

import { channelAPI } from "../../servers/api/channel";
import type { ChannelMeta } from "../types";
import type { ChannelParticipant } from "../../../shared/types/channel";
import { useReadReceiptStore } from "../store/readReceiptStore";
import { activeMentionQuery } from "../lib/mentions";
import { rememberMentionTarget } from "../lib/mentionTargets";

interface MentionsOptions {
  channel?: ChannelMeta;
  serverId?: string;
  currentUserId?: string;
  nameOverrides: Record<string, string>;
  text: string;
  setText: (value: string) => void;
  composerRef: React.RefObject<HTMLInputElement>;
}

export function useMentions({
  channel,
  serverId,
  currentUserId,
  nameOverrides,
  text,
  setText,
  composerRef,
}: MentionsOptions) {
  const [mentionQuery, setMentionQuery] = React.useState<{
    query: string;
    start: number;
  } | null>(null);
  const [mentionIndex, setMentionIndex] = React.useState(0);
  const [participants, setParticipants] = React.useState<ChannelParticipant[]>([]);
  const participantsChannelRef = React.useRef<string | null>(null);

  const channelMemberCount = useReadReceiptStore((s) =>
    channel ? Object.keys(s.channelReads[channel.id] ?? {}).length : 0,
  );
  React.useEffect(() => {
    participantsChannelRef.current = null;
  }, [channelMemberCount]);

  const ensureParticipants = React.useCallback(() => {
    if (!channel || !serverId) return;
    if (participantsChannelRef.current === channel.id) return;
    const channelId = channel.id;
    participantsChannelRef.current = channelId;
    setParticipants([]);
    void (async () => {
      try {
        const all: ChannelParticipant[] = [];
        const seen = new Set<string>();
        let page = 1;
        let totalPages = 1;
        do {
          const res = await channelAPI.listParticipants(
            serverId,
            channelId,
            page,
            50,
          );
          for (const p of res.participants) {
            if (seen.has(p.id)) continue;
            seen.add(p.id);
            all.push(p);
          }
          totalPages = res.pagination?.totalPages ?? 1;
          page += 1;
        } while (
          page <= totalPages &&
          participantsChannelRef.current === channelId
        );
        if (participantsChannelRef.current === channelId) {
          for (const p of all) rememberMentionTarget(p.name, p.id);
          setParticipants(all);
        }
      } catch {
        if (participantsChannelRef.current === channelId) {
          participantsChannelRef.current = null;
        }
      }
    })();
  }, [channel, serverId]);

  const updateMentionQuery = React.useCallback(
    (value: string, caret: number | null) => {
      if (!channel) {
        setMentionQuery(null);
        return;
      }
      const q = activeMentionQuery(value, caret ?? value.length);
      setMentionQuery(q);
      if (q) {
        setMentionIndex(0);
        ensureParticipants();
      }
    },
    [channel, ensureParticipants],
  );

  const mentionSuggestions = React.useMemo(() => {
    if (!mentionQuery) return [];
    const q = mentionQuery.query.toLowerCase();
    return participants
      .filter((p) => p.id !== currentUserId)
      .map((p) => ({ ...p, name: nameOverrides[p.id] ?? p.name }))
      .filter((p) => p.name.toLowerCase().startsWith(q))
      .slice(0, 8);
  }, [mentionQuery, participants, nameOverrides, currentUserId]);

  const applyMention = React.useCallback(
    (name: string) => {
      if (!mentionQuery) return;
      const el = composerRef.current;
      const caret = el?.selectionStart ?? text.length;
      const next =
        text.slice(0, mentionQuery.start) + "@" + name + " " + text.slice(caret);
      setText(next);
      setMentionQuery(null);
      const pos = mentionQuery.start + name.length + 2;
      requestAnimationFrame(() => {
        el?.focus();
        el?.setSelectionRange(pos, pos);
      });
    },
    [mentionQuery, text, composerRef, setText],
  );


  const resetParticipants = React.useCallback(() => {
    setParticipants([]);
    participantsChannelRef.current = null;
  }, []);

  return {
    resetParticipants,
    mentionQuery,
    setMentionQuery,
    mentionIndex,
    setMentionIndex,
    mentionSuggestions,
    participants,
    participantsChannelRef,
    ensureParticipants,
    updateMentionQuery,
    applyMention,
  };
}
