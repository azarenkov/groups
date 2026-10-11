import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { CommunityLogo } from "../../../shared/ui/Avatar";
import { currentLocale } from "../../../i18n";
import type { PublicProfile } from "../../home/api/home";
import type { Connection } from "../api/user";

const useMinuteTick = () => {
  const [, force] = React.useReducer((n: number) => n + 1, 0);
  React.useEffect(() => {
    const align = 60_000 - (Date.now() % 60_000);
    let interval: ReturnType<typeof setInterval> | null = null;
    const timeout = setTimeout(() => {
      force();
      interval = setInterval(force, 60_000);
    }, align);
    return () => {
      clearTimeout(timeout);
      if (interval) clearInterval(interval);
    };
  }, []);
};

const formatLocalTime = (timezone: string, locale: string): string | null => {
  if (!timezone) return null;
  try {
    const now = new Date();
    const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit", timeZone: timezone }).format(now);
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: timezone, timeZoneName: "shortOffset" }).formatToParts(now);
    const offset = parts.find((p) => p.type === "timeZoneName")?.value ?? "";
    return offset ? `${time} (${offset})` : time;
  } catch {
    return null;
  }
};

const formatMemberSince = (iso: string | null, locale: string): string | null => {
  if (!iso) return null;
  const d = new Date(iso);
  if (!Number.isFinite(d.getTime())) return null;
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" }).format(d);
};

interface ProfileFactsProps {
  profile: PublicProfile | null;
  connections: Connection[];
  bio: string;
  blockedMe: boolean;
}

const ProfileFacts: React.FC<ProfileFactsProps> = ({ profile, connections, bio, blockedMe }) => {
  const { t } = useTranslation("profile");
  useMinuteTick();
  const locale = currentLocale();
  const localTime = profile && !blockedMe ? formatLocalTime(profile.timezone, locale) : null;
  const memberSince = profile && !blockedMe ? formatMemberSince(profile.memberSince, locale) : null;
  return (
    <>
      <div className="profile-divider" />
      {bio && (
        <>
          <div className="profile-label">{t("about")}</div>
          <div className="profile-about">{bio}</div>
        </>
      )}
      <div className={"profile-meta" + (bio ? "" : " no-about")}>
        <div>
          <div className="profile-label">{t("memberSince")}</div>
          <div className="profile-value">{memberSince ?? "—"}</div>
        </div>
        {localTime && (
          <div>
            <div className="profile-label">{t("localTime")}</div>
            <div className="profile-value">{localTime}</div>
          </div>
        )}
      </div>
      {connections.length > 0 && (
        <div className="profile-mutual">
          <div className="profile-label">{t("connections")}</div>
          <div className="profile-chips">
            {connections.map((c) => {
              const inner = (
                <>
                  <Icon name={c.provider === "steam" ? "steam" : "github"} size={14} />
                  <span>{c.displayName || c.provider}</span>
                </>
              );
              return c.url ? (
                <a
                  key={c.provider}
                  className="profile-chip profile-conn"
                  href={c.url}
                  target="_blank"
                  rel="noreferrer"
                >
                  {inner}
                </a>
              ) : (
                <span key={c.provider} className="profile-chip profile-conn">
                  {inner}
                </span>
              );
            })}
          </div>
        </div>
      )}
      {profile && !blockedMe && profile.mutualCommunities.length > 0 && (
        <div className="profile-mutual">
          <div className="profile-label">{t("mutualCommunities", { count: profile.mutualCommunities.length })}</div>
          <div className="profile-chips">
            {profile.mutualCommunities.map((s) => (
              <span key={s.id} className="profile-chip" title={s.name}>
                <CommunityLogo communityId={s.id} name={s.name} accent={s.accentColor} className="profile-chip-logo" />
                <span>{s.name}</span>
              </span>
            ))}
          </div>
        </div>
      )}
    </>
  );
};

export default ProfileFacts;
