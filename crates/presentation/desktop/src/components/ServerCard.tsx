import React from "react";
import { Server } from "../types/server";
import "../styles/ServerCard.css";

interface ServerCardProps {
  server: Server;
  isSelected: boolean;
  onSelect: (serverId: string) => void;
}

const ServerCard: React.FC<ServerCardProps> = ({
  server,
  isSelected,
  onSelect,
}) => {
  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  };

  return (
    <div
      className={`server-card ${isSelected ? "selected" : ""}`}
      onClick={() => onSelect(server.id)}
    >
      <div className="server-card-header">
        <div className="server-icon">
          {server.name.charAt(0).toUpperCase()}
        </div>
        <div className="server-info">
          <h3 className="server-name">{server.name}</h3>
          {server.description && (
            <p className="server-description">{server.description}</p>
          )}
        </div>
      </div>
      <div className="server-card-footer">
        <span className="server-date">
          Created {formatDate(server.createdAt)}
        </span>
      </div>
    </div>
  );
};

export default ServerCard;
