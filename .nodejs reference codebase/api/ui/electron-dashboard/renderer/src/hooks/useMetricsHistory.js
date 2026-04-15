/**
 * Custom hook for tracking metrics history (CPU, RAM sparklines)
 */
import { useState, useCallback } from "react";
import { safeGet } from "./useSocketConnection";

const MAX_HISTORY_POINTS = 25;

export function useMetricsHistory() {
  const [cpuHistory, setCpuHistory] = useState([]);
  const [ramHistory, setRamHistory] = useState([]);

  const updateHistory = useCallback((newData) => {
    const cpuUsage = safeGet(newData, "system.cpu.usage", 0);
    const ramPercent = safeGet(newData, "system.memory.percent", 0);

    setCpuHistory((prev) => {
      const next =
        prev.length >= MAX_HISTORY_POINTS ? prev.slice(1) : [...prev];
      next.push(cpuUsage);
      return next;
    });

    setRamHistory((prev) => {
      const next =
        prev.length >= MAX_HISTORY_POINTS ? prev.slice(1) : [...prev];
      next.push(ramPercent);
      return next;
    });
  }, []);

  const clearHistory = useCallback(() => {
    setCpuHistory([]);
    setRamHistory([]);
  }, []);

  return {
    cpuHistory,
    ramHistory,
    updateHistory,
    clearHistory,
  };
}
