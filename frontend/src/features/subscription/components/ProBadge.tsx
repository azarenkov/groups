import React from "react";

import type { BadgeStyle, Tier } from "../lib/plans";

interface ProBadgeProps {
  tier: Tier;
  badgeStyle: BadgeStyle;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const ProBadge: React.FC<ProBadgeProps> = ({ tier, badgeStyle, size = "md", className }) => {
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
      {badgeStyle === "numbered" && (
        <span className="pro-badge-count">{tier === "premium" ? "02" : "01"}</span>
      )}
    </span>
  );
};

export default ProBadge;
