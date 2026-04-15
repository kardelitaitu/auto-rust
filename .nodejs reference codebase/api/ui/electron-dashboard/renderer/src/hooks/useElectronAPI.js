/**
 * Custom hook for Electron API integration
 */
import { useState, useCallback, useEffect } from "react";

export function useElectronAPI() {
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(false);
  const [isCompact, setIsCompact] = useState(false);
  const [config, setConfig] = useState(null);

  useEffect(() => {
    // Load initial config from Electron
    const loadConfig = async () => {
      if (window.electronAPI?.getConfig) {
        try {
          const cfg = await window.electronAPI.getConfig();
          setConfig(cfg);
        } catch (e) {
          console.warn("[Dashboard] Could not load Electron config:", e);
        }
      }
    };
    loadConfig();
  }, []);

  const toggleAlwaysOnTop = useCallback(() => {
    window.electronAPI?.toggleAlwaysOnTop();
    setIsAlwaysOnTop((prev) => !prev);
  }, []);

  const toggleCompact = useCallback(() => {
    if (isCompact) {
      window.electronAPI?.setWindowSize(1200, 800, false);
    } else {
      window.electronAPI?.setWindowSize(400, 600, true);
    }
    setIsCompact((prev) => !prev);
  }, [isCompact]);

  const showNotification = useCallback((title, body) => {
    window.electronAPI?.showNotification(title, body);
  }, []);

  return {
    isAlwaysOnTop,
    isCompact,
    config,
    toggleAlwaysOnTop,
    toggleCompact,
    showNotification,
    isElectron: !!window.electronAPI,
  };
}
