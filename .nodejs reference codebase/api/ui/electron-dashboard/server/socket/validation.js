/**
 * Payload validation functions for socket events
 */

const VALID_TASK_FIELDS = [
  "id",
  "taskName",
  "name",
  "command",
  "sessionId",
  "session",
  "timestamp",
  "status",
  "success",
  "error",
  "duration",
];
const VALID_SESSION_FIELDS = [
  "id",
  "status",
  "browser",
  "profile",
  "port",
  "ws",
  "lastSeen",
  "firstSeen",
];
const VALID_METRIC_FIELDS = ["twitter", "api", "browsers"];

/**
 * Validate a task object.
 * @param {Object} task - Task to validate
 * @returns {Object|null} - Validated task or null if invalid
 */
export function validateTask(task) {
  if (!task || typeof task !== "object") return null;
  const validated = {};
  for (const key of VALID_TASK_FIELDS) {
    if (task[key] !== undefined) validated[key] = task[key];
  }
  return Object.keys(validated).length > 0 ? validated : null;
}

/**
 * Validate a session object.
 * @param {Object} session - Session to validate
 * @returns {Object|null} - Validated session or null if invalid
 */
export function validateSession(session) {
  if (!session || typeof session !== "object") return null;
  const validated = {};
  for (const key of VALID_SESSION_FIELDS) {
    if (session[key] !== undefined) validated[key] = session[key];
  }
  return Object.keys(validated).length > 0 ? validated : null;
}

/**
 * Validate metrics object.
 * @param {Object} metrics - Metrics to validate
 * @returns {Object|null} - Validated metrics or null if invalid
 */
export function validateMetrics(metrics) {
  if (!metrics || typeof metrics !== "object") return null;
  const validated = {};
  if (metrics.twitter && typeof metrics.twitter === "object") {
    validated.twitter = metrics.twitter;
  }
  if (metrics.api && typeof metrics.api === "object") {
    validated.api = metrics.api;
  }
  if (metrics.browsers && typeof metrics.browsers === "object") {
    validated.browsers = metrics.browsers;
  }
  return Object.keys(validated).length > 0 ? validated : null;
}

/**
 * Validate a full payload with sessions, tasks, metrics.
 * @param {Object} payload - Payload to validate
 * @returns {Object|null} - Validated payload or null if invalid
 */
export function validatePayload(payload) {
  if (!payload || typeof payload !== "object") return null;
  const validated = {};
  if (Array.isArray(payload.sessions) && payload.sessions.length > 0) {
    validated.sessions = payload.sessions.map(validateSession).filter(Boolean);
  }
  if (Array.isArray(payload.recentTasks) && payload.recentTasks.length > 0) {
    validated.recentTasks = payload.recentTasks
      .map(validateTask)
      .filter(Boolean);
  }
  if (payload.metrics) {
    validated.metrics = validateMetrics(payload.metrics);
  }
  if (Array.isArray(payload.errors) && payload.errors.length > 0) {
    validated.errors = payload.errors.filter((e) => typeof e === "string");
  }
  return Object.keys(validated).length > 0 ? validated : null;
}
