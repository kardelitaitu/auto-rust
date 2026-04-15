/**
 * Dashboard Test Data Generator
 * Generates realistic data over time to test dashboard panels
 * Run: node api/tests/dashboard-data-generator.js
 * Run with custom port: PORT=3003 node api/tests/dashboard-data-generator.js
 * Run with duration: DURATION=30 node api/tests/dashboard-data-generator.js
 */

import { DashboardServer } from "../ui/electron-dashboard/dashboard.js";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function randomInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function generateSessions() {
  const sessionNames = [
    "Primary Session",
    "Secondary Session",
    "Worker-1",
    "Worker-2",
    "Bot-A",
    "Bot-B",
    "Twitter-Agent",
    "Scraper-01",
  ];

  const tasks = [
    "twitterLike",
    "twitterFollow",
    "twitterRetweet",
    "twitterReply",
    "twitterQuote",
    "twitterBookmark",
    "twitterTweet",
    "idle",
  ];

  const statuses = ["online", "idle", "offline"];

  const sessions = [];
  const numSessions = randomInt(3, 8);

  for (let i = 0; i < numSessions; i++) {
    const status = statuses[randomInt(0, 2)];
    sessions.push({
      id: `session-${Date.now()}-${i}`,
      name: sessionNames[i % sessionNames.length],
      status: status,
      activeWorkers: status === "online" ? randomInt(1, 5) : 0,
      totalWorkers: randomInt(1, 8),
      taskName:
        status === "online" ? tasks[randomInt(0, tasks.length - 1)] : null,
      createdAt: Date.now() - randomInt(60000, 3600000),
    });
  }

  return sessions;
}

function generateTwitterMetrics(previous) {
  const actions = previous?.twitter?.actions || {
    likes: 0,
    retweets: 0,
    replies: 0,
    quotes: 0,
    follows: 0,
    bookmarks: 0,
  };

  const shouldAdd = Math.random() > 0.3;
  if (shouldAdd) {
    const actionType = randomInt(0, 5);
    switch (actionType) {
      case 0:
        actions.likes += randomInt(1, 3);
        break;
      case 1:
        actions.retweets += randomInt(1, 2);
        break;
      case 2:
        actions.replies += randomInt(1, 2);
        break;
      case 3:
        actions.quotes += randomInt(0, 1);
        break;
      case 4:
        actions.follows += randomInt(1, 2);
        break;
      case 5:
        actions.bookmarks += randomInt(0, 1);
        break;
    }
  }

  actions.total =
    actions.likes +
    actions.retweets +
    actions.replies +
    actions.quotes +
    actions.follows +
    actions.bookmarks;

  return { twitter: { actions } };
}

function generateApiMetrics(previous) {
  const calls = (previous?.api?.calls || 0) + randomInt(0, 5);
  const failures =
    (previous?.api?.failures || 0) + (Math.random() > 0.8 ? 1 : 0);
  const successRate =
    calls > 0 ? Math.round(((calls - failures) / calls) * 100) : 100;
  const avgResponseTime = randomInt(150, 800);

  return { api: { calls, failures, successRate, avgResponseTime } };
}

function generateBrowserMetrics(previous) {
  const discovered = randomInt(2, 12);
  const connected = randomInt(1, Math.min(discovered, 6));

  return { browsers: { discovered, connected } };
}

function generateRecentTasks(previous) {
  const taskNames = [
    "twitterLike",
    "twitterFollow",
    "twitterRetweet",
    "twitterReply",
    "twitterQuote",
    "twitterBookmark",
    "twitterTweet",
    "login",
    "navigate",
  ];

  let tasks = previous?.recentTasks || [];

  if (Math.random() > 0.6) {
    const newTask = {
      taskName: taskNames[randomInt(0, taskNames.length - 1)],
      success: Math.random() > 0.15,
      duration: randomInt(500, 15000),
      timestamp: Date.now(),
    };
    tasks = [newTask, ...tasks].slice(0, 10);
  }

  return { recentTasks: tasks };
}

function generateQueueStatus() {
  return {
    queue: {
      queueLength: randomInt(0, 25),
      maxQueueSize: 500,
    },
  };
}

async function startDataGenerator() {
  const port = process.env.PORT ? parseInt(process.env.PORT) : 3002;
  console.log("[DataGenerator] Starting dashboard data generator...");
  console.log(
    "[DataGenerator] This will generate realistic data for dashboard panels",
  );
  console.log("[DataGenerator] Press Ctrl+C to stop\n");

  const dashboard = new DashboardServer(port, 2000);

  let cumulativeMetrics = {
    engineUptimeMs: 0,
    sessionUptimeMs: 0,
    clientConnectTime: Date.now(),
    totalTasksCompleted: 0,
    startTime: Date.now(),
  };

  let previousMetrics = {};
  let metricsHistory = [];

  await dashboard.start();
  console.log(
    `[DataGenerator] Dashboard server running on port ${dashboard.port}`,
  );

  let tickCount = 0;

  const interval = setInterval(() => {
    tickCount++;

    cumulativeMetrics.engineUptimeMs += 2000;
    cumulativeMetrics.sessionUptimeMs += 2000;

    const sessions = generateSessions();
    const queueStatus = generateQueueStatus();

    const twitterData = generateTwitterMetrics(previousMetrics);
    const apiData = generateApiMetrics(previousMetrics);
    const browserData = generateBrowserMetrics(previousMetrics);
    const tasksData = generateRecentTasks(previousMetrics);

    previousMetrics = {
      ...previousMetrics,
      ...twitterData,
      ...apiData,
      ...browserData,
      ...tasksData,
    };

    const payload = {
      sessions: sessions,
      ...queueStatus,
      metrics: {
        system: dashboard.getSystemMetrics(),
        ...twitterData,
        ...apiData,
        ...browserData,
      },
      ...tasksData,
      cumulative: cumulativeMetrics,
    };

    dashboard.updateMetrics(payload);

    if (tickCount % 5 === 0) {
      console.log(
        `[Tick ${tickCount}] Sessions: ${sessions.filter((s) => s.status === "online").length}/${sessions.length} | ` +
          `Queue: ${queueStatus.queue.queueLength} | ` +
          `Twitter: ${twitterData.twitter.actions.total} actions | ` +
          `API: ${apiData.api.calls} calls (${apiData.api.successRate}% success)`,
      );
    }
  }, 2000);

  const duration = process.env.DURATION ? parseInt(process.env.DURATION) : null;
  if (duration) {
    console.log(`[DataGenerator] Will run for ${duration} seconds...\n`);
    setTimeout(() => {
      console.log("\n[DataGenerator] Duration complete, stopping...");
      clearInterval(interval);
      dashboard.stop();
      process.exit(0);
    }, duration * 1000);
  }

  process.on("SIGINT", () => {
    console.log("\n[DataGenerator] Stopping...");
    clearInterval(interval);
    dashboard.stop();
    process.exit(0);
  });
}

startDataGenerator().catch((err) => {
  console.error("[DataGenerator] Error:", err);
  process.exit(1);
});
