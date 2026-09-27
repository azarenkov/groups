import React from "react";

import { useProfileModalStore } from "../store/profileModalStore";

interface ProfileLinkProps {
  userId: string;
  className?: string;
  style?: React.CSSProperties;
  children?: React.ReactNode;
}

const ProfileLink: React.FC<ProfileLinkProps> = ({
  userId,
  className,
  style,
  children,
}) => {
  const open = (e: React.SyntheticEvent) => {
    e.stopPropagation();
    useProfileModalStore.getState().open(userId);
  };
  return (
    <span
      role="link"
      tabIndex={0}
      className={"profile-link" + (className ? ` ${className}` : "")}
      style={style}
      onClick={open}
      onKeyDown={(e) => {
        if (e.key !== "Enter" && e.key !== " ") return;
        e.preventDefault();
        open(e);
      }}
    >
      {children}
    </span>
  );
};

export default ProfileLink;
