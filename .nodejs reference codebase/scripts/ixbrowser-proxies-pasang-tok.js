/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const APIProfileUpdate = "http://127.0.0.1:53200/api/v2/profile-update";
const APIProfileList = "http://127.0.0.1:53200/api/v2/profile-list";
const PROXIES_FILE = path.join(__dirname, "proxies.txt");

// Read and parse proxies from file
function loadProxies() {
  try {
    const content = fs.readFileSync(PROXIES_FILE, "utf-8");
    const lines = content
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"));

    const proxies = lines
      .map((line) => {
        const parts = line.split(":");
        if (parts.length === 4) {
          return {
            host: parts[0],
            port: parts[1],
            user: parts[2],
            pass: parts[3],
          };
        }
        return null;
      })
      .filter((p) => p !== null);

    return proxies;
  } catch (error) {
    console.error("[pasang-tok] Error reading proxies.txt:", error.message);
    return [];
  }
}

// Robust fetch with retry
async function fetchWithRetry(url, options, retries = 3, backoff = 1000) {
  for (let i = 0; i < retries; i++) {
    try {
      const response = await fetch(url, options);
      if (!response.ok) throw new Error(`HTTP Error: ${response.status}`);
      return await response.json();
    } catch (error) {
      if (i === retries - 1) throw error;
      console.warn(
        `[pasang-tok] API Attempt ${i + 1} failed: ${error.message}. Retrying in ${backoff}ms...`,
      );
      await new Promise((resolve) => setTimeout(resolve, backoff));
      backoff *= 2; // Exponential backoff
    }
  }
}

async function getTotalProfiles() {
  try {
    const data = await fetchWithRetry(APIProfileList, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ page: 1, limit: 10000 }),
    });

    if (data.data && typeof data.data.total === "number") {
      return data.data.total;
    } else if (data.data && Array.isArray(data.data.list)) {
      return data.data.list.length;
    } else if (Array.isArray(data.data)) {
      return data.data.length;
    }

    return 0;
  } catch (error) {
    console.error("[pasang-tok] Error fetching profile count:", error.message);
    return 0;
  }
}

// Assign proxy directly using proxy_config
async function assignProxyToProfile(profileId, proxy) {
  const payload = {
    profile_id: parseInt(profileId),
    proxy_config: {
      proxy_mode: 2,
      proxy_check_line: "global_line",
      proxy_id: "",
      proxy_type: "socks5",
      proxy_ip: proxy.host,
      proxy_port: proxy.port,
      proxy_user: proxy.user,
      proxy_password: proxy.pass,
      ip_detection: "0",
      traffic_package_ip_policy: false,
      country: "",
      city: "",
      gateway: "Default",
      proxy_service: "general",
      proxy_data_format_type: "txt",
      proxy_data_txt_format: "ip:port",
      proxy_extraction_method: "invalid",
      proxy_url: "",
      use_system_proxy: "1",
      enable_bypass: "0",
      bypass_list: "",
    },
  };

  // DEBUG: Log first 3 requests
  if (profileId <= 3) {
    console.log(
      `[pasang-tok] DEBUG - Request for Profile ${profileId}:`,
      JSON.stringify(payload, null, 2),
    );
  }

  try {
    const data = await fetchWithRetry(APIProfileUpdate, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    // DEBUG: Log first 3 responses
    if (profileId <= 3) {
      console.log(
        `[pasang-tok] DEBUG - Response for Profile ${profileId}:`,
        JSON.stringify(data),
      );
    }

    if (data.error && data.error.code === 0) {
      console.log(
        `[pasang-tok] ✓ Profile ${profileId} → ${proxy.host}:${proxy.port}`,
      );
      return true;
    } else {
      console.warn(
        `[pasang-tok] ✗ Profile ${profileId}: ${data.error ? data.error.message : "Unknown"}`,
      );
      return false;
    }
  } catch (error) {
    console.warn(`[pasang-tok] ✗ Profile ${profileId}: ${error.message}`);
    return false;
  }
}

function parseTargetProfiles(args, totalProfiles) {
  if (args.length === 0) {
    // Default: 1 to total
    return Array.from({ length: totalProfiles }, (_, i) => i + 1);
  }

  const targets = new Set();
  args.forEach((arg) => {
    if (arg.includes("-")) {
      const [start, end] = arg.split("-").map(Number);
      if (!isNaN(start) && !isNaN(end)) {
        for (let i = start; i <= end; i++) {
          if (i > 0 && i <= totalProfiles) targets.add(i);
        }
      }
    } else {
      const id = parseInt(arg);
      if (!isNaN(id) && id > 0 && id <= totalProfiles) {
        targets.add(id);
      }
    }
  });

  return Array.from(targets).sort((a, b) => a - b);
}

async function main() {
  console.log("[pasang-tok] ========================================");
  console.log("[pasang-tok] ASSIGN PROXIES (ROBUST + CLI SUPPORT)");
  console.log("[pasang-tok] ========================================\n");

  // STEP 1: Load proxies from file
  const proxies = loadProxies();
  if (proxies.length === 0) {
    console.log("[pasang-tok] No proxies found in proxies.txt");
    return;
  }
  console.log(`[pasang-tok] Loaded ${proxies.length} proxies from file`);

  // STEP 2: Get total profiles to validate bounds
  const totalCount = await getTotalProfiles();
  if (totalCount === 0) {
    console.log("[pasang-tok] No profiles found or API error.");
    return;
  }
  console.log(`[pasang-tok] Total Profiles in Browser: ${totalCount}`);

  // STEP 3: Parse CLI Arguments
  const cliArgs = process.argv.slice(2);
  const targetIds = parseTargetProfiles(cliArgs, totalCount);

  if (targetIds.length === 0) {
    console.log("[pasang-tok] No valid target profiles to process.");
    return;
  }

  console.log(`[pasang-tok] Targets: ${targetIds.length} profiles to update\n`);

  // STEP 4: Assign proxies
  const BATCH_SIZE = 1; // Keeping 1 for maximum safety, can be increased
  const STABILITY_DELAY = 200; // ms between requests

  let successCount = 0;
  for (let i = 0; i < targetIds.length; i += BATCH_SIZE) {
    const batch = [];
    for (let j = 0; j < BATCH_SIZE && i + j < targetIds.length; j++) {
      const profileId = targetIds[i + j];
      // Map Profile ID N to Proxy Line N (0-indexed)
      const proxyIndex = (profileId - 1) % proxies.length;
      const proxy = proxies[proxyIndex];

      batch.push(
        assignProxyToProfile(profileId, proxy).then((success) => {
          if (success) successCount++;
        }),
      );
    }

    await Promise.all(batch);

    if (i + BATCH_SIZE < targetIds.length) {
      await new Promise((resolve) => setTimeout(resolve, STABILITY_DELAY));
    }

    // Progress indicator
    if ((i + BATCH_SIZE) % 50 === 0 || i + BATCH_SIZE >= targetIds.length) {
      const current = Math.min(i + BATCH_SIZE, targetIds.length);
      console.log(
        `[pasang-tok] Progress: ${current}/${targetIds.length} (${Math.round((current / targetIds.length) * 100)}%)`,
      );
    }
  }

  console.log(`\n[pasang-tok] ========================================`);
  console.log(
    `[pasang-tok] ✓ COMPLETE: ${successCount}/${targetIds.length} profiles updated`,
  );
  console.log(`[pasang-tok] ========================================`);
}

main();
