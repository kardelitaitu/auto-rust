/**
 * Auto-AI Framework - Proprietary Software
 * Copyright (c) 2025 gantengmaksimal - All Rights Reserved
 * Unauthorized copying, distribution, or modification prohibited
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load profiles safely from ../data/ relative to utils/
const profilesPath = path.join(
  __dirname,
  "../../data/twitterActivityProfiles.json",
);
const generatorPath = path.join(__dirname, "generateProfiles.js");
let PROFILES = [];

/**
 * Loads profiles from the JSON file
 * @returns {boolean} True if profiles were loaded successfully
 */
function loadProfiles() {
  try {
    if (fs.existsSync(profilesPath)) {
      const data = fs.readFileSync(profilesPath, "utf8");
      const loaded = JSON.parse(data);

      if (Array.isArray(loaded)) {
        const validProfiles = loaded.filter((p) => {
          if (!p) {
            console.warn(
              "[WARN] Profile validation issues: Empty profile entry",
            );
            return false;
          }
          if (!p.id || !p.timings || !p.probabilities) {
            console.warn(
              `[WARN] Profile validation issues: Missing required fields for ${p?.id || "unknown"}`,
            );
            return false;
          }

          // Validate scrollPause mean exists as it's critical for selection
          if (
            !p.timings.scrollPause ||
            typeof p.timings.scrollPause.mean !== "number"
          ) {
            console.warn(
              `[WARN] Profile validation issues: Invalid scrollPause timings for ${p.id}`,
            );
            return false;
          }

          return true;
        });

        if (validProfiles.length < loaded.length) {
          console.warn(
            `[WARN] ${loaded.length - validProfiles.length} invalid profiles were skipped.`,
          );
        }

        PROFILES = validProfiles;
        return PROFILES.length > 0;
      }
      PROFILES = [];
      return false;
    }
    PROFILES = [];
    return false;
  } catch (e) {
    console.error("[ERROR] Failed to load profiles", e);
    PROFILES = [];
    return false;
  }
}

/**
 * Runs the profile generator script
 * @returns {boolean} True if generation was successful
 */
function generateProfiles() {
  try {
    console.log("[ProfileManager] No profiles found. Auto-generating...");
    // Run generateProfiles.js synchronously
    execSync(`node "${generatorPath}"`, {
      cwd: path.join(__dirname, ".."),
      stdio: "inherit", // Show output
    });
    console.log("[ProfileManager] Profile generation complete.");
    return true;
  } catch (e) {
    console.error("[ProfileManager] Failed to generate profiles:", e.message);
    return false;
  }
}

/**
 * Ensures profiles are loaded, generating them if necessary
 * @returns {boolean} True if profiles are available
 */
function ensureProfilesLoaded() {
  if (PROFILES.length > 0) return true;

  // Try loading first
  if (loadProfiles()) return true;

  // If loading failed, try generating
  if (generateProfiles()) {
    // Retry loading after generation
    return loadProfiles();
  }

  return false;
}

// Initial load attempt
loadProfiles();
if (PROFILES.length === 0) {
  console.warn("[WARN] Profiles not found. Will auto-generate on first use.");
}

export const profileManager = {
  getStarter: () => {
    if (!ensureProfilesLoaded()) {
      throw new Error(
        "No profiles loaded and auto-generation failed. Please run utils/generateProfiles.js manually.",
      );
    }

    const fast = PROFILES.filter((p) => p.timings.scrollPause.mean < 2500);
    const pool = fast.length > 0 ? fast : PROFILES;

    return pool[Math.floor(Math.random() * pool.length)];
  },

  getFatiguedVariant: (currentMean) => {
    if (!ensureProfilesLoaded()) {
      return null; // Graceful degradation for fatigue variant
    }

    const candidates = PROFILES.filter(
      (p) => p.timings.scrollPause.mean > currentMean * 1.4,
    );
    if (candidates.length === 0) return null;
    return candidates[Math.floor(Math.random() * candidates.length)];
  },

  getById: (profileId) => {
    if (!ensureProfilesLoaded()) {
      throw new Error(
        "No profiles loaded and auto-generation failed. Please run utils/generateProfiles.js manually.",
      );
    }

    const profile = PROFILES.find((p) => p.id === profileId);

    if (!profile) {
      const availableIds = PROFILES.map((p) => p.id).join(", ");
      throw new Error(
        `Profile "${profileId}" not found. Available profiles: ${availableIds}`,
      );
    }

    return profile;
  },

  /**
   * Manually reload profiles from disk
   * @returns {boolean} True if reload was successful
   */
  reload: () => {
    return loadProfiles();
  },

  /**
   * Get count of loaded profiles
   * @returns {number} Number of profiles currently loaded
   */
  getCount: () => {
    return PROFILES.length;
  },

  /**
   * Async version of getStarter
   * @returns {Promise<Object>}
   */
  getStarterAsync: async () => {
    return profileManager.getStarter();
  },

  /**
   * Async version of getById
   * @param {string} profileId
   * @returns {Promise<Object>}
   */
  getByIdAsync: async (profileId) => {
    return profileManager.getById(profileId);
  },

  /**
   * Async version of reload
   * @returns {Promise<boolean>}
   */
  reloadAsync: async () => {
    return loadProfiles();
  },

  /**
   * Resets the internal profiles array (For testing only)
   */
  reset: () => {
    PROFILES = [];
  },
};
