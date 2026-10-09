import React from "react";

import { useBadge } from "../hooks/useBadge";
import EmojiStatusIcon from "./EmojiStatusIcon";
import ProBadge from "./ProBadge";

interface UserBadgeProps {
  userId: string | null | undefined;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const UserBadge: React.FC<UserBadgeProps> = ({ userId, size, className }) => {
  const badge = useBadge(userId);
  if (!badge) return null;
  return (
    <>
      <ProBadge
        tier={badge.tier}
        badgeStyle={badge.badgeStyle}
        memberNumber={badge.memberNumber}
        size={size}
        className={className}
      />
      <EmojiStatusIcon status={badge.emojiStatus} size={size} />
    </>
  );
};

export default UserBadge;
