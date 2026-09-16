import React from "react";

export const ReadTicks: React.FC<{ read: boolean }> = ({ read }) => (
  <svg
    className="rc-icon"
    viewBox="0 0 15 11"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {read ? (
      <>
        <path d="M1 6.2 L3.5 8.7 L7.8 3" />
        <path d="M5.4 6.2 L7.9 8.7 L12.2 3" />
      </>
    ) : (
      <path d="M2.4 6.2 L5 8.7 L9.6 3" />
    )}
  </svg>
);

export default ReadTicks;
