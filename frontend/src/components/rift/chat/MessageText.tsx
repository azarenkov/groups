import React from "react";

import { useAuthStore } from "../../../store/authStore";
import { URL_REGEX } from "../../../utils/links";
import { MENTION_REGEX } from "../../../utils/mentions";

export const renderMentions = (
  text: string,
  keyPrefix: string,
): React.ReactNode[] => {
  const selfName = useAuthStore.getState().user?.name?.toLowerCase();
  const parts: React.ReactNode[] = [];
  let last = 0;
  let i = 0;
  for (const match of text.matchAll(MENTION_REGEX)) {
    const start = (match.index ?? 0) + match[1].length;
    const name = match[2];
    if (start > last) parts.push(text.slice(last, start));
    const isSelf = !!selfName && name.toLowerCase() === selfName;
    parts.push(
      <span
        key={`${keyPrefix}m${i++}`}
        className={"msg-mention" + (isSelf ? " self" : "")}
      >
        @{name}
      </span>,
    );
    last = start + name.length + 1;
  }
  if (parts.length === 0) return [text];
  if (last < text.length) parts.push(text.slice(last));
  return parts;
};

export const renderWithLinks = (text: string): React.ReactNode => {
  const parts: React.ReactNode[] = [];
  let last = 0;
  let i = 0;
  for (const match of text.matchAll(URL_REGEX)) {
    const idx = match.index ?? 0;
    if (idx > last) parts.push(...renderMentions(text.slice(last, idx), `s${i}`));
    parts.push(
      <a
        key={`l${i++}`}
        href={match[0]}
        target="_blank"
        rel="noopener noreferrer"
        className="msg-link"
      >
        {match[0]}
      </a>,
    );
    last = idx + match[0].length;
  }
  if (parts.length === 0) return renderMentions(text, "t");
  if (last < text.length) parts.push(...renderMentions(text.slice(last), "e"));
  return parts;
};
