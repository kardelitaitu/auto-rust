/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

const profileInput = process.argv[2];

if (!profileInput) {
  console.error(
    "[ixbrowser-change-fingerprint-config.js] Error: Please provide a profile ID or range.",
  );
  console.error(
    "Usage: node ixbrowser-change-fingerprint-config.js <profile_id>",
  );
  console.error(
    "       node ixbrowser-change-fingerprint-config.js <start>-<end>",
  );
  console.error("Examples: node ixbrowser-change-fingerprint-config.js 5");
  console.error("          node ixbrowser-change-fingerprint-config.js 1-30");
  process.exit(1);
}

// Parse profile ID(s) - support both single ID and range (e.g., "1-30")
let profileIds = [];
if (profileInput.includes("-")) {
  const [start, end] = profileInput
    .split("-")
    .map((num) => parseInt(num.trim()));
  if (isNaN(start) || isNaN(end) || start > end || start < 1) {
    console.error(
      "[ixbrowser-change-fingerprint-config.js] Error: Invalid range format.",
    );
    console.error(
      "Range must be in format: <start>-<end> where start <= end and both are positive numbers.",
    );
    process.exit(1);
  }
  for (let i = start; i <= end; i++) {
    profileIds.push(i);
  }
  console.log(
    `[ixbrowser-change-fingerprint-config.js] Processing profiles ${start} to ${end} (${profileIds.length} profiles)`,
  );
} else {
  const singleId = parseInt(profileInput);
  if (isNaN(singleId) || singleId < 1) {
    console.error(
      "[ixbrowser-change-fingerprint-config.js] Error: Invalid profile ID.",
    );
    process.exit(1);
  }
  profileIds.push(singleId);
}

const APILocal = "http://127.0.0.1:53200/api/v2/profile-update";

// Database of Real MacBook configurations (M1, M2, M3, M4)
const macBookFingerprints = [
  // ================= M1 FAMILY =================
  {
    model: "MacBook Air (M1, 2020) 8GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "8",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
  },
  {
    model: "MacBook Air (M1, 2020) 16GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
  },
  {
    model: "MacBook Pro (13-inch, M1, 2020) 8GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "8",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
  },
  {
    model: "MacBook Pro (13-inch, M1, 2020) 16GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M1 Pro, 2021) 16GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M1 Pro, 2021) 32GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "10",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M1 Pro, 2021) 16GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "10",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M1 Pro, 2021) 32GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "10",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M1 Max, 2021) 32GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "10",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Max, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M1 Max, 2021) 64GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "10",
    device_memory: "64",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M1 Max, Unspecified Version)",
  },
  // ================= M2 FAMILY =================
  {
    model: "MacBook Air (13-inch, M2, 2022) 8GB",
    resolving_power: "2560,1664",
    hardware_concurrency: "8",
    device_memory: "8",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Air (13-inch, M2, 2022) 16GB",
    resolving_power: "2560,1664",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Air (13-inch, M2, 2022) 24GB",
    resolving_power: "2560,1664",
    hardware_concurrency: "8",
    device_memory: "24",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Air (15-inch, M2, 2023) 8GB",
    resolving_power: "2880,1864",
    hardware_concurrency: "8",
    device_memory: "8",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Air (15-inch, M2, 2023) 16GB",
    resolving_power: "2880,1864",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Air (15-inch, M2, 2023) 24GB",
    resolving_power: "2880,1864",
    hardware_concurrency: "8",
    device_memory: "24",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Pro (13-inch, M2, 2022) 16GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Pro (13-inch, M2, 2022) 24GB",
    resolving_power: "2560,1600",
    hardware_concurrency: "8",
    device_memory: "24",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M2 Pro, 2023) 16GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "10",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M2 Pro, 2023) 32GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "12",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M2 Pro, 2023) 16GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "12",
    device_memory: "16",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M2 Pro, 2023) 32GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "12",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Pro, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M2 Max, 2023) 32GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "12",
    device_memory: "32",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Max, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M2 Max, 2023) 64GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "12",
    device_memory: "64",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Max, Unspecified Version)",
  },
  {
    model: "MacBook Pro (14-inch, M2 Max, 2023) 96GB",
    resolving_power: "3024,1964",
    hardware_concurrency: "12",
    device_memory: "96",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Max, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M2 Max, 2023) 64GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "12",
    device_memory: "64",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Max, Unspecified Version)",
  },
  {
    model: "MacBook Pro (16-inch, M2 Max, 2023) 96GB",
    resolving_power: "3456,2234",
    hardware_concurrency: "12",
    device_memory: "96",
    webgl_info:
      "ANGLE (Apple, ANGLE Metal Renderer: Apple M2 Max, Unspecified Version)",
  },
];

function getRandomFingerprint() {
  // Weights:
  // Air: 40 (Highest frequency)
  // Pro (Base/Standard): 10
  // Pro (Pro chip): 5
  // Pro (Max chip): 1 (Lowest frequency)

  // Calculate weights dynamically
  const weightedAndIndices = macBookFingerprints.map((fp, index) => {
    let weight = 10; // Default base weight for Pro standard

    if (fp.model.includes("Air")) {
      weight = 40;
    } else if (fp.model.includes("Max")) {
      weight = 1;
    } else if (fp.model.includes("Pro")) {
      // Check if it's a "Pro" chip (e.g. M1 Pro, M2 Pro) vs just "MacBook Pro" machine name with M1/M2/M3 base
      // Our model strings are like "MacBook Pro (14-inch, M3 Pro, 2023)..."
      // If it matches "M\d Pro" or "M\d Max" (handled above)
      // Regex to find "M[digit] Pro"
      if (/M\d\sPro/.test(fp.model)) {
        weight = 5;
      } else {
        // Base model Pro (e.g. M1, M2, M3 standard chip in Pro body)
        weight = 10;
      }
    }

    return { index, weight };
  });

  const totalWeight = weightedAndIndices.reduce(
    (sum, item) => sum + item.weight,
    0,
  );
  let randomValue = Math.random() * totalWeight;

  for (const item of weightedAndIndices) {
    randomValue -= item.weight;
    if (randomValue <= 0) {
      return macBookFingerprints[item.index];
    }
  }

  // Fallback
  return macBookFingerprints[0];
}

async function updateProfile(profileId) {
  const randomConfig = getRandomFingerprint();

  const payload = {
    profile_id: profileId,
    fingerprint_config: {
      resolving_power_type: "2", // Always 2
      resolving_power: randomConfig.resolving_power,
      webgl_data_type: "2", // Always 2
      webgl_factory: "Google Inc. (Apple)", // Always Google Inc. (Apple)
      webgl_info: randomConfig.webgl_info,
      hardware_concurrency: randomConfig.hardware_concurrency,
      device_memory: randomConfig.device_memory,
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
        `[ixbrowser-change-fingerprint-config.js] ✓ Profile ${profileId} updated with ${randomConfig.model}.`,
      );
      console.log(
        `  - CPU: ${randomConfig.hardware_concurrency} cores | RAM: ${randomConfig.device_memory} GB | Resolution: ${randomConfig.resolving_power.replace(",", "x")}`,
      );
      return { success: true, profileId };
    } else {
      // 102009 or similar codes might indicate issues.
      // If profile is open, usually it returns an error or just fails to update.
      // We will log it as a Warning to avoid pipeline failure noise.
      console.warn(
        `[ixbrowser-change-fingerprint-config.js] ✗ Warning: Could not update profile ${profileId}.`,
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
        `[ixbrowser-change-fingerprint-config.js] ✗ Warning: Cannot connect to ixBrowser API (is it running?). Skipping update for profile ${profileId}.`,
      );
      return { success: false, profileId, error: "API not running" };
    } else {
      console.warn(
        `[ixbrowser-change-fingerprint-config.js] ✗ Warning: Network error updating profile ${profileId}: ${error.message}`,
      );
      return { success: false, profileId, error: error.message };
    }
  }
}

async function processAllProfiles() {
  console.log(
    `[ixbrowser-change-fingerprint-config.js] Starting batch update...`,
  );
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
  console.log("[ixbrowser-change-fingerprint-config.js] BATCH UPDATE SUMMARY");
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
