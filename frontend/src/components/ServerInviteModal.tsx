import React, { useCallback, useEffect, useState } from "react";
import Icon from "./rift/Icon";
import { serverAPI } from "../api/server";
import { ServerInvite } from "../types/server";
import "../styles/rift.css";

interface ServerInviteModalProps {
  isOpen: boolean;
  serverId: string | null;
  serverName: string;
  onClose: () => void;
}

const ServerInviteModal: React.FC<ServerInviteModalProps> = ({
  isOpen,
  serverId,
  serverName,
  onClose,
}) => {
  const [invites, setInvites] = useState<ServerInvite[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!serverId) return;
    setLoading(true);
    setError(null);
    try {
      const list = await serverAPI.listInvites(serverId);
      setInvites(list);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't load invites");
    } finally {
      setLoading(false);
    }
  }, [serverId]);

  useEffect(() => {
    if (!isOpen) return;
    load();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [isOpen, load, onClose]);

  useEffect(() => {
    if (!isOpen) {
      setInvites([]);
      setError(null);
      setCopied(null);
    }
  }, [isOpen]);

  if (!isOpen || !serverId) return null;

  const handleCreate = async () => {
    setCreating(true);
    setError(null);
    try {
      const invite = await serverAPI.createInvite(serverId);
      setInvites((prev) => [invite, ...prev]);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't create invite");
    } finally {
      setCreating(false);
    }
  };

  const handleRevoke = async (inviteId: string) => {
    try {
      await serverAPI.revokeInvite(serverId, inviteId);
      setInvites((prev) => prev.filter((i) => i.id !== inviteId));
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't revoke invite");
    }
  };

  const handleCopy = (code: string) => {
    navigator.clipboard?.writeText(code).catch(() => {});
    setCopied(code);
    window.setTimeout(() => setCopied((c) => (c === code ? null : c)), 1500);
  };

  return (
    <div className="modal-overlay" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="m-head">
          <h3>Invite people to {serverName}</h3>
          <button type="button" className="close" onClick={onClose} title="Close">
            <Icon name="x" size={14} />
          </button>
        </div>
        <div className="m-body">
          <button
            type="button"
            className="btn primary"
            onClick={handleCreate}
            disabled={creating}
            style={{ width: "100%" }}
          >
            {creating ? <span className="loading-spinner" /> : "Generate invite code"}
          </button>

          {error && <div className="m-error">{error}</div>}

          <div className="invite-list">
            {loading && <div className="invite-empty">Loading…</div>}
            {!loading && invites.length === 0 && (
              <div className="invite-empty">No active invites yet.</div>
            )}
            {invites.map((inv) => (
              <div key={inv.id} className="invite-row">
                <code className="invite-code">{inv.code}</code>
                <button
                  type="button"
                  className="invite-btn"
                  title="Copy code"
                  onClick={() => handleCopy(inv.code)}
                >
                  <Icon name={copied === inv.code ? "check" : "link"} size={13} />
                  {copied === inv.code ? "Copied" : "Copy"}
                </button>
                <button
                  type="button"
                  className="invite-btn danger"
                  title="Revoke invite"
                  onClick={() => handleRevoke(inv.id)}
                >
                  <Icon name="x" size={13} />
                </button>
              </div>
            ))}
          </div>
        </div>
        <div className="m-foot">
          <button type="button" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
};

export default ServerInviteModal;
