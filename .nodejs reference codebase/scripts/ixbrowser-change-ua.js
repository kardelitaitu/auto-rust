/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

const profileInput = process.argv[2];

if (!profileInput) {
  console.error(
    "[ixbrowser-change-ua.js] Error: Please provide a profile ID or range.",
  );
  console.error("Usage: node ixbrowser-change-ua.js <profile_id>");
  console.error("       node ixbrowser-change-ua.js <start>-<end>");
  console.error("Examples: node ixbrowser-change-ua.js 5");
  console.error("          node ixbrowser-change-ua.js 1-30");
  process.exit(1);
}

// Parse profile ID(s) - support both single ID and range (e.g., "1-30")
let profileIds = [];
if (profileInput.includes("-")) {
  const [start, end] = profileInput
    .split("-")
    .map((num) => parseInt(num.trim()));
  if (isNaN(start) || isNaN(end) || start > end || start < 1) {
    console.error("[ixbrowser-change-ua.js] Error: Invalid range format.");
    console.error(
      "Range must be in format: <start>-<end> where start <= end and both are positive numbers.",
    );
    process.exit(1);
  }
  for (let i = start; i <= end; i++) {
    profileIds.push(i);
  }
  console.log(
    `[ixbrowser-change-ua.js] Processing profiles ${start} to ${end} (${profileIds.length} profiles)`,
  );
} else {
  const singleId = parseInt(profileInput);
  if (isNaN(singleId) || singleId < 1) {
    console.error("[ixbrowser-change-ua.js] Error: Invalid profile ID.");
    process.exit(1);
  }
  profileIds.push(singleId);
}

const APILocal = "http://127.0.0.1:53200/api/v2/profile-update";

// Chrome versions to randomize
const CHROME_VERSIONS = [
  "142.0.7444.147",
  "142.0.7444.163",
  "142.0.7444.164",
  "142.0.7444.158",
  "142.0.7444.103",
  "142.0.7444.138",
  "142.0.7444.148",
  "142.0.7444.141",
  "142.0.7444.145",
  "142.0.7444.105",
  "142.0.7444.142",
  "142.0.7444.133",
  "142.0.7444.143",
  "142.0.7444.165",
  "142.0.7444.146",
  "142.0.7444.139",
  "142.0.7444.160",
  "142.0.7444.144",
  "142.0.7444.131",
  "142.0.7444.161",
  "142.0.7444.106",
  "142.0.7444.159",
];

// User-Agent base template
const UA_TEMPLATE =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{VERSION} Safari/537.36";

function getRandomUserAgent() {
  const randomVersion =
    CHROME_VERSIONS[Math.floor(Math.random() * CHROME_VERSIONS.length)];
  return UA_TEMPLATE.replace("{VERSION}", randomVersion);
}

async function updateProfile(profileId) {
  const randomUA = getRandomUserAgent();

  // Extract Chrome version from UA string for logging
  const chromeVersionMatch = randomUA.match(/Chrome\/([\d.]+)/);
  const chromeVersion = chromeVersionMatch ? chromeVersionMatch[1] : "Unknown";

  const payload = {
    profile_id: profileId,
    fingerprint_config: {
      ua_info: randomUA,
    },
  };

  try {
    const response = await fetch(APILocal, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    const data = await response.json();

    if (data.error && data.error.code === 0) {
      console.log(
        `[ixbrowser-change-ua.js] ✓ Profile ${profileId} updated with Chrome ${chromeVersion}`,
      );
      return { success: true, profileId };
    } else {
      // Log as warning to avoid pipeline failure noise
      console.warn(
        `[ixbrowser-change-ua.js] ✗ Warning: Could not update profile ${profileId}.`,
      );
      console.warn(
        `  Reason: ${data.error ? data.error.message : JSON.stringify(data)}`,
      );
      return {
        success: false,
        profileId,
        error: data.error ? data.error.message : "Unknown error",
      };
    }
  } catch (error) {
    // Handle connection refused (API not running) or other network issues gracefully
    if (error.code === "ECONNREFUSED") {
      console.warn(
        `[ixbrowser-change-ua.js] ✗ Warning: Cannot connect to ixBrowser API (is it running?). Skipping update for profile ${profileId}.`,
      );
      return { success: false, profileId, error: "API not running" };
    } else {
      console.warn(
        `[ixbrowser-change-ua.js] ✗ Warning: Network error updating profile ${profileId}: ${error.message}`,
      );
      return { success: false, profileId, error: error.message };
    }
  }
}

async function processAllProfiles() {
  console.log(`[ixbrowser-change-ua.js] Starting batch User-Agent update...`);
  console.log("");

  const results = [];

  for (let i = 0; i < profileIds.length; i++) {
    const profileId = profileIds[i];
    const progress = `[${i + 1}/${profileIds.length}]`;
    console.log(`${progress} Processing profile ${profileId}...`);

    const result = await updateProfile(profileId);
    results.push(result);

    // Add a small delay between requests to avoid overwhelming the API
    if (i < profileIds.length - 1) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }

  // Print summary
  console.log("");
  console.log("=".repeat(60));
  console.log("[ixbrowser-change-ua.js] BATCH UPDATE SUMMARY");
  console.log("=".repeat(60));

  const successful = results.filter((r) => r.success).length;
  const failed = results.filter((r) => !r.success).length;

  console.log(`Total profiles processed: ${results.length}`);
  console.log(`✓ Successful: ${successful}`);
  console.log(`✗ Failed: ${failed}`);

  if (failed > 0) {
    console.log("");
    console.log("Failed profiles:");
    results
      .filter((r) => !r.success)
      .forEach((r) => {
        console.log(`  - Profile ${r.profileId}: ${r.error}`);
      });
  }

  console.log("=".repeat(60));
}

processAllProfiles();
