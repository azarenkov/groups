import React from "react";

interface LoadingDotsProps {
  className?: string;
}

const LoadingDots: React.FC<LoadingDotsProps> = ({ className }) => (
  <span className={"loading-spinner" + (className ? ` ${className}` : "")} aria-hidden="true">
    <span />
    <span />
    <span />
  </span>
);

export default LoadingDots;
