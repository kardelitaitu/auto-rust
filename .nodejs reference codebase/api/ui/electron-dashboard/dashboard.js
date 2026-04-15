/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 *
 * @deprecated This file is maintained for backward compatibility.
 * New code should import from './server/index.js' directly.
 */

// Re-export everything from the new modular server
export {
  DashboardServer,
  startStandaloneServer,
  isAuthenticated,
  withAuth,
  sanitizeLogString,
  sanitizeObject,
  validateTask,
  validateSession,
  validateMetrics,
  validatePayload,
} from "./server/index.js";

// Default export for backward compatibility
export { DashboardServer as default } from "./server/index.js";
