import React from "react";

import { colorForId, glyphFor, initialFor } from "../lib/avatar";
import { useAvatarUrl } from "../lib/avatarCache";

interface AvatarBaseProps {
  className?: string;
  style?: React.CSSProperties;
  title?: string;
  children?: React.ReactNode;
  onClick?: React.MouseEventHandler<HTMLDivElement>;
}

const AvatarImage: React.FC<{ url: string; alt: string }> = ({ url, alt }) => {
  const [broken, setBroken] = React.useState(false);
  React.useEffect(() => setBroken(false), [url]);
  if (broken) return null;
  return (
    <img
      className="avatar-img"
      src={url}
      alt={alt}
      draggable={false}
      onError={() => setBroken(true)}
    />
  );
};

export const UserAvatar: React.FC<
  AvatarBaseProps & { userId?: string | null; name: string }
> = ({ userId, name, className, style, title, children, onClick }) => {
  const url = useAvatarUrl("user", userId);
  return (
    <div
      className={className}
      title={title}
      onClick={onClick}
      style={{ background: colorForId(userId ?? ""), position: "relative", ...style }}
    >
      {initialFor(name)}
      {url && <AvatarImage url={url} alt={name} />}
      {children}
    </div>
  );
};

export const CommunityLogo: React.FC<
  AvatarBaseProps & { communityId?: string | null; name: string; accent?: string | null }
> = ({ communityId, name, accent, className, style, title, children, onClick }) => {
  const url = useAvatarUrl("community", communityId);
  return (
    <div
      className={className}
      title={title}
      onClick={onClick}
      style={{
        background: accent || colorForId(communityId ?? ""),
        position: "relative",
        ...style,
      }}
    >
      {glyphFor(name)}
      {url && <AvatarImage url={url} alt={name} />}
      {children}
    </div>
  );
};
