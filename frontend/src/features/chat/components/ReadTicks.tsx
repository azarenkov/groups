import React from "react";

export const ReadTicks: React.FC<{ read: boolean }> = ({ read }) =>
  read ? (
    <svg
      className="rc-icon double"
      viewBox="0 0 16 10"
      width={16}
      height={10}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M1 5.5 3.5 8 8.5 2" />
      <path d="M7 7.5 7.5 8 12.5 2" />
    </svg>
  ) : (
    <svg
      className="rc-icon"
      viewBox="0 0 12 10"
      width={12}
      height={10}
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M2 5.5 4.5 8 9.5 2" />
    </svg>
  );

export default ReadTicks;
