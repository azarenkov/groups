import React from "react";

import { useBadge } from "../hooks/useBadge";
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
    <ProBadge
      tier={badge.tier}
      badgeStyle={badge.badgeStyle}
      size={size}
      className={className}
    />
  );
};

export default UserBadge;
