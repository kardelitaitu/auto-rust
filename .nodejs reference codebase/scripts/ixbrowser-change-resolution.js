/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

const profileInput = process.argv[2];

if (!profileInput) {
  console.error(
    "[change-resolution.js] Error: Please provide a profile ID or range.",
  );
  console.error("Usage: node ixbrowser-change-resolution.js <profile_id>");
  console.error("       node ixbrowser-change-resolution.js <start>-<end>");
  console.error("Examples: node ixbrowser-change-resolution.js 5");
  console.error("          node ixbrowser-change-resolution.js 1-30");
  process.exit(1);
}

// Parse profile ID(s) - support both single ID and range (e.g., "1-30")
let profileIds = [];
if (profileInput.includes("-")) {
  const [start, end] = profileInput
    .split("-")
    .map((num) => parseInt(num.trim()));
  if (isNaN(start) || isNaN(end) || start > end || start < 1) {
    console.error("[change-resolution.js] Error: Invalid range format.");
    console.error(
      "Range must be in format: <start>-<end> where start <= end and both are positive numbers.",
    );
    process.exit(1);
  }
  for (let i = start; i <= end; i++) {
    profileIds.push(i);
  }
  console.log(
    `[change-resolution.js] Processing profiles ${start} to ${end} (${profileIds.length} profiles)`,
  );
} else {
  const singleId = parseInt(profileInput);
  if (isNaN(singleId) || singleId < 1) {
    console.error("[change-resolution.js] Error: Invalid profile ID.");
    process.exit(1);
  }
  profileIds.push(singleId);
}

const APILocalUpdate = "http://127.0.0.1:53200/api/v2/profile-update";
const APILocalList = "http://127.0.0.1:53200/api/v2/profile-list";

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
  // Weighted selection: Air (40) > Pro Base (10) > Pro Pro (5) > Max (1)
  const weightedAndIndices = macBookFingerprints.map((fp, index) => {
    let weight = 10;
    if (fp.model.includes("Air")) {
      weight = 40;
    } else if (fp.model.includes("Max")) {
      weight = 1;
    } else if (fp.model.includes("Pro")) {
      if (/M\d\sPro/.test(fp.model)) {
        weight = 5;
      } else {
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
  return macBookFingerprints[0];
}

async function updateProfile(profileId, config) {
  const payload = {
    profile_id: parseInt(profileId),
    fingerprint_config: {
      resolving_power_type: "2",
      resolving_power: config.resolving_power,
      hardware_concurrency: config.hardware_concurrency,
      device_memory: config.device_memory,
    },
  };

  try {
    const response = await fetch(APILocalUpdate, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    const data = await response.json();

    if (data.error && data.error.code === 0) {
      const resolution = config.resolving_power.replace(",", "x");
      console.log(
        `[change-resolution.js] ✓ Profile ${profileId} updated with ${config.model}.`,
      );
      console.log(
        `  - CPU: ${config.hardware_concurrency} cores | RAM: ${config.device_memory} GB | Resolution: ${resolution}`,
      );
      return { success: true, profileId };
    } else {
      console.warn(
        `[change-resolution.js] ✗ Warning: Could not update profile ${profileId}.`,
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
    if (error.code === "ECONNREFUSED") {
      console.warn(
        `[change-resolution.js] ✗ Warning: Cannot connect to ixBrowser API (is it running?). Skipping update for profile ${profileId}.`,
      );
      return { success: false, profileId, error: "API not running" };
    } else {
      console.warn(
        `[change-resolution.js] ✗ Warning: Network error updating profile ${profileId}: ${error.message}`,
      );
      return { success: false, profileId, error: error.message };
    }
  }
}

async function _getTotalProfiles() {
  try {
    const response = await fetch(APILocalList, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ page: 1, limit: 10000 }),
    });
    const data = await response.json();

    // Try different response structures
    if (data.data && typeof data.data.total === "number") {
      return data.data.total;
    } else if (data.data && Array.isArray(data.data.list)) {
      return data.data.list.length;
    } else if (Array.isArray(data.data)) {
      return data.data.length;
    }

    console.error("Could not parse total from API response");
    return 0;
  } catch (error) {
    console.error("Error fetching profile count:", error.message);
    return 0;
  }
}

async function processAllProfiles() {
  console.log(`[change-resolution.js] Starting batch update...`);
  console.log("");

  const results = [];

  for (let i = 0; i < profileIds.length; i++) {
    const profileId = profileIds[i];
    const progress = `[${i + 1}/${profileIds.length}]`;
    console.log(`${progress} Processing profile ${profileId}...`);

    const randomConfig = getRandomFingerprint();
    const result = await updateProfile(profileId, randomConfig);
    results.push(result);

    // Add a small delay between requests to avoid overwhelming the API
    if (i < profileIds.length - 1) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }

  // Print summary
  console.log("");
  console.log("=".repeat(60));
  console.log("[change-resolution.js] BATCH UPDATE SUMMARY");
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
