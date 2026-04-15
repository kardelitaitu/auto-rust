/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

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
        `[resolusi+cpu+ram] ✓ Profile ${profileId}: ${config.model} | ${config.hardware_concurrency} cores | ${config.device_memory} GB | ${resolution}`,
      );
    } else {
      console.warn(
        `[resolusi+cpu+ram] ✗ Profile ${profileId}: ${data.error ? data.error.message : "Unknown error"}`,
      );
    }
  } catch (error) {
    if (error.code === "ECONNREFUSED") {
      console.warn(
        `[resolusi+cpu+ram] ✗ Profile ${profileId}: API not running`,
      );
    } else {
      console.warn(
        `[resolusi+cpu+ram] ✗ Profile ${profileId}: ${error.message}`,
      );
    }
  }
}

async function getTotalProfiles() {
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

async function main() {
  console.log("[resolusi+cpu+ram] Fetching total profile count...");

  const total = await getTotalProfiles();

  if (total === 0) {
    console.log("[resolusi+cpu+ram] No profiles found or API error.");
    return;
  }

  console.log(`[resolusi+cpu+ram] Total Profiles: ${total}`);
  console.log(
    `[resolusi+cpu+ram] Starting bulk update for Profile IDs 1 to ${total}...`,
  );
  console.log(`[resolusi+cpu+ram] Concurrency: 2 parallel requests\n`);

  const CONCURRENCY = 2;
  let completed = 0;

  // Process profiles in batches with concurrency limit
  for (let i = 1; i <= total; i += CONCURRENCY) {
    const batch = [];

    // Create a batch of up to CONCURRENCY profiles
    for (let j = 0; j < CONCURRENCY && i + j <= total; j++) {
      const profileId = i + j;
      const randomConfig = getRandomFingerprint();
      batch.push(updateProfile(profileId, randomConfig));
    }

    // Wait for all profiles in this batch to complete
    await Promise.all(batch);
    completed += batch.length;

    // Progress indicator every 100 profiles
    if (completed % 100 === 0 || completed === total) {
      console.log(
        `[resolusi+cpu+ram] Progress: ${completed}/${total} (${Math.round((completed / total) * 100)}%)`,
      );
    }
  }

  console.log(`\n[resolusi+cpu+ram] ✓ All ${total} profiles processed.`);
}

main();
