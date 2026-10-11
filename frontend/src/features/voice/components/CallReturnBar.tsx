import React from "react";
import { useCallStore } from "../store/callStore";

export const CallErrorToast: React.FC = () => {
  const error = useCallStore((s) => s.callError);
  const setCallError = useCallStore((s) => s.setCallError);
  React.useEffect(() => {
    if (!error) return;
    const timer = window.setTimeout(() => setCallError(null), 4000);
    return () => window.clearTimeout(timer);
  }, [error, setCallError]);
  if (!error) return null;
  return (
    <div className="call-error-toast" role="alert">
      {error}
    </div>
  );
};

