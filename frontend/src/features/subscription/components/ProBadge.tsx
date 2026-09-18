import React from "react";

import { formatMemberNumber } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";

interface ProBadgeProps {
  tier: Tier;
  badgeStyle: BadgeStyle;
  size?: "sm" | "md" | "lg";
  className?: string;
  memberNumber?: number | null;
}

const ProBadge: React.FC<ProBadgeProps> = ({ tier, badgeStyle, size = "md", className, memberNumber }) => {
  if (tier === "free") return null;
  const label = tier === "premium" ? "PREMIUM" : "PRO";
  return (
    <span
      className={
        `pro-badge pro-badge-${badgeStyle} pro-badge-${size}` +
        (className ? ` ${className}` : "")
      }
      title={tier === "premium" ? "Rift Premium" : "Rift Pro"}
    >
      {label}
      {badgeStyle === "numbered" && memberNumber != null && (
        <span className="pro-badge-count">{formatMemberNumber(memberNumber)}</span>
      )}
    </span>
  );
};

export default ProBadge;
