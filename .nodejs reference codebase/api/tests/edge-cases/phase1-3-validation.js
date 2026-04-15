/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

/**
 * Comprehensive Module Test Suite
 * Validates all Phase 1-3 modules for correctness and parallel safety
 *
 * Run with: node tests/phase1-3-validation.js
 */

import { humanTiming } from "../utils/timing.js";
import { sessionPhases } from "../utils/session-phases.js";
import { engagementLimits } from "../utils/engagement-limits.js";
import { sentimentGuard } from "../utils/sentiment-guard.js";
import {
  navigationDiversity,
  NAV_STATES,
} from "../utils/navigation-diversity.js";
import { EntropyController } from "../utils/entropyController.js";
import { ActionOrchestrator } from "../utils/actionOrchestrator.js";

console.log("=".repeat(80));
console.log("AI Twitter Activity - Phase 1-3 Module Validation");
console.log("=".repeat(80));
console.log("");

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`✅ ${name}`);
    passed++;
  } catch (error) {
    console.log(`❌ ${name}`);
    console.log(`   Error: ${error.message}`);
    failed++;
  }
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

// ============================================================================
// PHASE 1: CORE MODULES
// ============================================================================

console.log("--- Phase 1: Core Modules ---");
console.log("");

// Test 1.1: Human Timing - Gaussian Distribution
test("Human Timing: gaussianRandom produces valid values", () => {
  const mean = 1000;
  const stdev = 200;
  const values = [];

  for (let i = 0; i < 1000; i++) {
    values.push(humanTiming.gaussianRandom(mean, stdev));
  }

  const avg = values.reduce((a, b) => a + b, 0) / values.length;
  const variance =
    values.reduce((sum, val) => sum + Math.pow(val - avg, 2), 0) /
    values.length;
  const actualStdev = Math.sqrt(variance);

  assert(
    avg > mean * 0.8 && avg < mean * 1.2,
    `Average ${avg} too far from mean ${mean}`,
  );
  assert(
    actualStdev > stdev * 0.5 && actualStdev < stdev * 1.5,
    `Stdev ${actualStdev} too far from expected ${stdev}`,
  );
});

test("Human Timing: humanDelay with jitter/pause/burst", () => {
  const base = 1000;
  const delay = humanTiming.humanDelay(base);

  assert(delay >= 50, `Delay ${delay} below minimum`);
  assert(delay > base * 0.5, `Delay ${delay} unreasonably fast`);
});

test("Human Timing: gaussianInRange respects bounds", () => {
  const values = [];

  for (let i = 0; i < 100; i++) {
    values.push(humanTiming.gaussianInRange(5000, 2000, 1000, 15000));
  }

  const inRange = values.every((v) => v >= 1000 && v <= 15000);
  assert(inRange, "Some values outside bounds");
});

test("Human Timing: getReadingTime returns valid content types", () => {
  const types = ["quick", "text", "image", "video", "thread", "longThread"];

  for (const type of types) {
    const time = humanTiming.getReadingTime(type);
    assert(time > 0, `Reading time for ${type} should be positive`);
  }
});

// Test 1.2: Session Phases
test("Session Phases: getSessionPhase returns correct phases", () => {
  const warmup = sessionPhases.getSessionPhase(50000, 600000);
  const active = sessionPhases.getSessionPhase(120000, 600000);
  const cooldown = sessionPhases.getSessionPhase(500000, 600000);

  assert(warmup === "warmup", `Expected warmup, got ${warmup}`);
  assert(active === "active", `Expected active, got ${active}`);
  assert(cooldown === "cooldown", `Expected cooldown, got ${cooldown}`);
});

test("Session Phases: getPhaseModifier returns multipliers", () => {
  const replyMod = sessionPhases.getPhaseModifier("reply", "warmup");
  const likeMod = sessionPhases.getPhaseModifier("like", "active");
  const diveMod = sessionPhases.getPhaseModifier("dive", "cooldown");

  assert(
    replyMod > 0 && replyMod <= 1,
    `Reply modifier ${replyMod} out of range`,
  );
  assert(likeMod === 1.0, `Active like modifier should be 1.0`);
  assert(
    diveMod > 0 && diveMod < 1,
    `Cooldown dive modifier ${diveMod} should be < 1`,
  );
});

test("Session Phases: getPhaseDescription returns strings", () => {
  const warmupDesc = sessionPhases.getPhaseDescription("warmup");
  const activeDesc = sessionPhases.getPhaseDescription("active");
  const cooldownDesc = sessionPhases.getPhaseDescription("cooldown");

  assert(typeof warmupDesc === "string", "Warmup description should be string");
  assert(warmupDesc.length > 0, "Warmup description should not be empty");
  assert(typeof activeDesc === "string", "Active description should be string");
  assert(
    typeof cooldownDesc === "string",
    "Cooldown description should be string",
  );
});

// Test 1.3: Engagement Limits
test("Engagement Limits: createEngagementTracker", () => {
  const tracker = engagementLimits.createEngagementTracker();

  assert(tracker.canPerform("replies"), "Should be able to reply");
  assert(tracker.canPerform("likes"), "Should be able to like");
  assert(tracker.getRemaining("replies") > 0, "Should have remaining replies");
});

test("Engagement Limits: record action and update count", () => {
  const tracker = engagementLimits.createEngagementTracker();

  assert(tracker.record("replies"), "Should record first reply");
  assert(tracker.stats.replies === 1, "Replies should be 1");
  assert(tracker.getRemaining("replies") === 2, "Remaining should be 2");
});

test("Engagement Limits: prevent exceeding limits", () => {
  const tracker = engagementLimits.createEngagementTracker({
    replies: 3,
    likes: 5,
  });

  tracker.record("replies");
  tracker.record("replies");
  tracker.record("replies");

  assert(
    !tracker.canPerform("replies"),
    "Should not be able to reply at limit",
  );
  assert(tracker.isExhausted("replies"), "Replies should be exhausted");
});

test("Engagement Limits: getSummary format", () => {
  const tracker = engagementLimits.createEngagementTracker();
  const summary = tracker.getSummary();

  assert(typeof summary === "string", "Summary should be string");
  assert(summary.includes("replies:"), "Summary should include replies");
});

// ============================================================================
// PHASE 2: COGNITIVE MODULES
// ============================================================================

console.log("");
console.log("--- Phase 2: Cognitive Modules ---");
console.log("");

// Test 2.1: Sentiment Guard
test("Sentiment Guard: detect negative content", () => {
  const negativeTweet = "Rest in peace my dear friend. You will be missed.";
  const result = sentimentGuard.analyzeSentiment(negativeTweet);

  assert(result.isNegative, "Should detect RIP tweet as negative");
  assert(result.shouldSkipLikes, "Should skip likes on negative");
  assert(result.shouldSkipRetweets, "Should skip retweets on negative");
});

test("Sentiment Guard: allow neutral content", () => {
  const neutralTweet = "Just had an amazing coffee! ☕";
  const result = sentimentGuard.analyzeSentiment(neutralTweet);

  // Should not be marked as negative (no death/tragedy keywords)
  assert(!result.isNegative, "Should not detect neutral tweet as negative");
  assert(result.score < 0.15, "Neutral tweet should have low score");
});

test("Navigation: state transitions", () => {
  const nav = navigationDiversity.createNavigationManager();

  nav.transition("clickAvatar");
  assert(
    nav.getCurrentState() === NAV_STATES.PROFILE,
    "Should transition to PROFILE",
  );

  nav.transition("clickBack");
  assert(nav.getCurrentState() === NAV_STATES.FEED, "Should return to FEED");
});

test("Navigation: return path", () => {
  const nav = navigationDiversity.createNavigationManager();

  nav.transition("clickAvatar");
  nav.transition("clickPinned");

  const path = nav.getReturnPath();

  assert(path.length > 0, "Should have return path");
  assert(path.includes("clickBack"), "Return path should include clickBack");
});

test("Sentiment Guard: tragedy keywords", () => {
  const tragedy =
    "Tragedy strikes our community today. Our thoughts go out to the victims.";
  const result = sentimentGuard.analyzeSentiment(tragedy);

  assert(
    result.categories.some((c) => c.name === "tragedy"),
    "Should detect tragedy category",
  );
});

test("Sentiment Guard: scam detection", () => {
  const scam = "My account was hacked! Someone stole my password!";
  const result = sentimentGuard.analyzeSentiment(scam);

  assert(
    result.categories.some((c) => c.name === "scam"),
    "Should detect scam category",
  );
});

test("Sentiment Guard: getSafeActions", () => {
  const sad = "So sad about what happened. Heartbreaking news.";
  const safe = sentimentGuard.getSafeActions(sad);

  assert(safe.canExpand, "Should allow expanding negative content");
  assert(!safe.canLike, "Should block likes on sad content");
  assert(!safe.canRetweet, "Should block retweets on sad content");
});

test("Sentiment Guard: formatSentimentReport", () => {
  const negative = "RIP. Gone too soon.";
  const report = sentimentGuard.formatSentimentReport(negative);

  assert(report.includes("NEGATIVE"), "Report should indicate negative");
});

// Test 2.2: Navigation Diversity
test("Navigation: createNavigationManager", () => {
  const nav = navigationDiversity.createNavigationManager();

  assert(nav.getCurrentState() === NAV_STATES.FEED, "Should start in FEED");
});

test("Navigation: state transitions", () => {
  const nav = navigationDiversity.createNavigationManager();

  nav.transition("clickAvatar");
  assert(
    nav.getCurrentState() === NAV_STATES.PROFILE,
    "Should transition to PROFILE",
  );

  nav.transition("clickBack");
  assert(nav.getCurrentState() === NAV_STATES.FEED, "Should return to FEED");
});

test("Navigation: rabbit hole depth tracking", () => {
  const nav = navigationDiversity.createNavigationManager();

  assert(nav.getDepth() === 0, "Should start at depth 0");

  nav.incrementDepth();
  nav.incrementDepth();

  assert(nav.getDepth() === 2, "Should be at depth 2");
});

test("Navigation: return path", () => {
  const nav = navigationDiversity.createNavigationManager();

  nav.transition("clickAvatar");
  nav.transition("clickPinned");

  const path = nav.getReturnPath();

  assert(path.length > 0, "Should have return path");
  assert(path.includes("clickBack"), "Return path should include clickBack");
});

// ============================================================================
// PHASE 3: ADVANCED MODULES
// ============================================================================

console.log("");
console.log("--- Phase 3: Advanced Modules ---");
console.log("");

// Test 3.1: Entropy Controller - Parallel Safety
test("EntropyController: create new instance with sessionId", () => {
  const entropy1 = new EntropyController({ sessionId: "browser-1" });
  const entropy2 = new EntropyController({ sessionId: "browser-2" });

  assert(
    entropy1.sessionId !== entropy2.sessionId,
    "Instances should have different IDs",
  );
  assert(entropy1.sessionId === "browser-1", "Session ID should match");
});

test("EntropyController: separate fatigue tracking", () => {
  const entropy1 = new EntropyController({ sessionId: "browser-1" });
  const entropy2 = new EntropyController({ sessionId: "browser-2" });

  assert(entropy1.fatigueActive === false, "Should start without fatigue");
  assert(entropy2.fatigueActive === false, "Both should start without fatigue");
});

test("EntropyController: retryDelay returns valid timing", () => {
  const entropy = new EntropyController();
  const delay = entropy.retryDelay();

  assert(typeof delay === "number", "Retry delay should be a number");
  assert(!isNaN(delay), "Retry delay should not be NaN");
  assert(delay >= 500, "Retry delay should be >= 500ms");
  assert(delay <= 30000, "Retry delay should be <= 30000ms");
});

// Test 3.2: Action Orchestrator - Parallel Safety
test("ActionOrchestrator: create new instance", () => {
  const orch1 = new ActionOrchestrator({ sessionId: "browser-1" });
  const orch2 = new ActionOrchestrator({ sessionId: "browser-2" });

  assert(
    orch1.sessionId !== orch2.sessionId,
    "Instances should have different IDs",
  );
  assert(orch1.history.length === 0, "Should start with empty history");
});

test("ActionOrchestrator: track separate histories", () => {
  const orch1 = new ActionOrchestrator({ sessionId: "browser-1" });
  const orch2 = new ActionOrchestrator({ sessionId: "browser-2" });

  orch1.record("TIMELINE_BROWSE");
  orch1.record("TWEET_DIVE");

  assert(orch1.history.length === 2, "Browser 1 should have 2 actions");
  assert(orch2.history.length === 0, "Browser 2 should have 0 actions");
});

// ============================================================================
// PARALLEL SAFETY TESTS
// ============================================================================

console.log("");
console.log("--- Parallel Safety Tests ---");
console.log("");

test("Parallel: independent entropy instances", () => {
  const instances = [];

  for (let i = 0; i < 5; i++) {
    instances.push(new EntropyController({ sessionId: `browser-${i}` }));
  }

  const sessionIds = instances.map((e) => e.sessionId);
  const uniqueIds = new Set(sessionIds);

  assert(uniqueIds.size === 5, "All instances should have unique session IDs");

  instances.forEach((entropy) => {
    assert(
      entropy.sessionStart > 0,
      `Session ${entropy.sessionId} should have start time`,
    );
  });
});

test("Parallel: independent orchestrator instances", () => {
  const instances = [];

  for (let i = 0; i < 5; i++) {
    const orch = new ActionOrchestrator({ sessionId: `browser-${i}` });
    orch.record("TWEET_DIVE");
    instances.push(orch);
  }

  instances.forEach((orch) => {
    assert(
      orch.history.length === 1,
      `Orchestrator ${orch.sessionId} should have own history`,
    );
  });
});

test("Parallel: engagement limits isolation", () => {
  const trackers = [];

  for (let i = 0; i < 3; i++) {
    const tracker = engagementLimits.createEngagementTracker({ replies: 3 });

    for (let j = 0; j < 2; j++) {
      tracker.record("replies");
    }

    trackers.push(tracker);
  }

  trackers.forEach((tracker) => {
    assert(
      tracker.stats.replies === 2,
      "Each tracker should have independent count",
    );
  });
});

// ============================================================================
// SUMMARY
// ============================================================================

console.log("");
console.log("=".repeat(80));
console.log("VALIDATION SUMMARY");
console.log("=".repeat(80));
console.log(`✅ Passed: ${passed}`);
console.log(`❌ Failed: ${failed}`);
console.log("");

if (failed === 0) {
  console.log(
    "🎉 ALL TESTS PASSED! Phase 1-3 modules are ready for production.",
  );
  console.log("");
  console.log("Next steps:");
  console.log("1. Integrate modules into ai-twitterActivity.js");
  console.log("2. Run with: node main.js ai-twitterActivity");
  console.log("3. Monitor logs for [Phase], [Sentiment], [Motor] tags");
} else {
  console.log("⚠️ Some tests failed. Please review the errors above.");
  process.exit(1);
}

console.log("");
