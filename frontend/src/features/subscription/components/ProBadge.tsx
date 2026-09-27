import React from "react";

import { DEFAULT_BADGE_STYLE, formatMemberNumber } from "../lib/plans";
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
  const plan = tier === "premium" ? "Rift Premium" : "Rift Pro";
  if (badgeStyle === "number_only" && memberNumber != null) {
    const number = formatMemberNumber(memberNumber);
    return (
      <span
        className={
          `pro-badge pro-badge-number-only pro-badge-${size}` + (className ? ` ${className}` : "")
        }
        title={`${plan} #${number}`}
      >
        #{number}
      </span>
    );
  }
  const shownStyle = badgeStyle === "number_only" ? DEFAULT_BADGE_STYLE : badgeStyle;
  return (
    <span
      className={
        `pro-badge pro-badge-${shownStyle} pro-badge-${size}` +
        (className ? ` ${className}` : "")
      }
      title={plan}
    >
      {label}
      {shownStyle === "numbered" && memberNumber != null && (
        <span className="pro-badge-count">{formatMemberNumber(memberNumber)}</span>
      )}
    </span>
  );
};

export default ProBadge;
