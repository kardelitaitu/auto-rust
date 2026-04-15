/**
 * Custom hook for Socket.io connection management
 */
import { useState, useEffect, useRef, useCallback } from "react";
import io from "socket.io-client";

const safeGet = (obj, path, defaultValue = "N/A") => {
  const keys = path.split(".");
  let result = obj;
  for (const key of keys) {
    if (result == null) return defaultValue;
    result = result[key];
  }
  return result ?? defaultValue;
};

export function useSocketConnection() {
  const [data, setData] = useState({
    sessions: [],
    queue: { queueLength: 0 },
    metrics: {},
    recentTasks: [],
    system: {},
    cumulative: {},
  });
  const [status, setStatus] = useState("connecting");
  const [serverUrl, setServerUrl] = useState(null);
  const socketRef = useRef(null);

  useEffect(() => {
    const initSocket = async () => {
      let url = "http://localhost:3001";

      if (window.electronAPI?.onConfigLoaded) {
        window.electronAPI.onConfigLoaded((config) => {
          window.__DASHBOARD_CONFIG__ = config;
        });
      }

      const params = new URLSearchParams(window.location.search);
      const urlParam = params.get("server");
      if (urlParam) {
        url = urlParam;
      } else if (window.electronAPI?.getConfig) {
        try {
          const config = await window.electronAPI.getConfig();
          if (config?.serverUrl) {
            url = config.serverUrl;
          }
        } catch (e) {
          console.warn("[Dashboard] Could not get config from Electron:", e);
        }
      }

      setServerUrl(url);
      console.log("[Dashboard] Connecting to:", url);

      const socket = io(url, {
        transports: ["websocket"],
        reconnection: true,
        reconnectionAttempts:
          window.__DASHBOARD_CONFIG__?.client?.reconnectionAttempts || 10,
        reconnectionDelay:
          window.__DASHBOARD_CONFIG__?.client?.reconnectionDelay || 2000,
      });

      socket.on("connect", () => setStatus("online"));
      socket.on("disconnect", () => setStatus("offline"));
      socket.on("connect_error", (err) => {
        console.warn("[Dashboard] Connection error:", err.message);
        setStatus("no-connection");
      });

      socketRef.current = socket;

      socket.on("metrics", (newData) => {
        setData(newData);
      });

      return () => socket.close();
    };

    initSocket();
  }, []);

  const emit = useCallback((event, data) => {
    if (socketRef.current?.connected) {
      socketRef.current.emit(event, data);
    }
  }, []);

  const requestUpdate = useCallback(() => {
    if (socketRef.current?.connected) {
      socketRef.current.emit("requestUpdate");
    }
  }, []);

  return {
    data,
    status,
    serverUrl,
    socket: socketRef.current,
    emit,
    requestUpdate,
  };
}

export { safeGet };
