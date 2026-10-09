import React from "react";

import { useBadge } from "../hooks/useBadge";
import EmojiStatusIcon from "./EmojiStatusIcon";
import { useActiveEmojiStatus } from "../lib/emojiStatus";
import ProBadge from "./ProBadge";

interface UserBadgeProps {
  userId: string | null | undefined;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const UserBadge: React.FC<UserBadgeProps> = ({ userId, size, className }) => {
  const badge = useBadge(userId);
  const status = useActiveEmojiStatus(badge?.emojiStatus);
  if (!badge) return null;
  if (status) return <EmojiStatusIcon status={status} size={size} />;
  return (
    <ProBadge
      tier={badge.tier}
      badgeStyle={badge.badgeStyle}
      memberNumber={badge.memberNumber}
      size={size}
      className={className}
    />
  );
};

export default UserBadge;
