/**
 * .SYNOPSIS
 * Benchmarks the vLLM PagedAttention throughput via Docker Model Runner.
 * * .DESCRIPTION
 * Spawns concurrent fetch requests (CPS) to measure parallel Data Processing Speed (DPS).
 * Enforces strict logging and session feedback.
 */

const BASE_URL = "http://localhost:8000";
let API_ENDPOINT = "";
let TARGET_MODEL = "ai/gemma3-vllm:1B";
let CONCURRENT_AGENTS = 5;

async function discoverModel() {
  const endpoints = ["/v1", "/engines/v1"];
  console.log(`[DISCOVERY] Probing vLLM artifacts at ${BASE_URL}...`);

  for (const prefix of endpoints) {
    try {
      const res = await fetch(`${BASE_URL}${prefix}/models`);
      if (res.ok) {
        const data = await res.json();
        const models = data.data.map((m) => m.id);
        console.log(`[DISCOVERY] Found active endpoint: ${prefix}`);
        console.log(`[DISCOVERY] Available models: ${models.join(", ")}`);

        // If target model isn't found, pick the first one
        if (!models.includes(TARGET_MODEL)) {
          console.warn(
            `[DISCOVERY] ${TARGET_MODEL} not found. Defaulting to ${models[0]}`,
          );
          TARGET_MODEL = models[0];
        }

        API_ENDPOINT = `${BASE_URL}${prefix}/chat/completions`;
        return true;
      }
    } catch {
      // Continue probing
    }
  }
  return false;
}

async function spawnAgent(agentId, prompt) {
  const startTime = performance.now();

  try {
    const response = await fetch(API_ENDPOINT, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: TARGET_MODEL,
        messages: [{ role: "user", content: prompt }],
        max_tokens: 512,
        temperature: 0.1,
      }),
    });

    if (!response.ok) {
      const errorBody = await response.text();
      throw new Error(`HTTP ${response.status}: ${errorBody}`);
    }

    const data = await response.json();
    const endTime = performance.now();

    const durationSec = (endTime - startTime) / 1000;
    const generatedTokens = data.usage?.completion_tokens || 0;
    const tokensPerSec = generatedTokens / durationSec;

    return {
      Agent: `Node-${agentId}`,
      Damage: parseFloat(tokensPerSec.toFixed(2)),
      Tokens: generatedTokens,
      Time: parseFloat(durationSec.toFixed(2)),
    };
  } catch (error) {
    console.error(`[AGENT-${agentId} ERROR] Systemic failure:`, error.message);
    return null;
  }
}

async function orchestrateBenchmark() {
  const discovered = await discoverModel();
  if (!discovered) {
    console.error(
      "[FATAL] No valid vLLM endpoints discovered. Ensure Model Runner is active.",
    );
    return;
  }

  console.log(
    `[INIT] Dispatching ${CONCURRENT_AGENTS} parallel sequences to ${TARGET_MODEL}...`,
  );

  // Standardized prompt to test Radix/Paged cache hit rate
  const systemPrompt =
    "Explain the architectural advantages of modular system design in 3 paragraphs.";

  const agentPromises = Array.from({ length: CONCURRENT_AGENTS }, (_, i) =>
    spawnAgent(i + 1, systemPrompt),
  );

  const results = await Promise.all(agentPromises);
  const validResults = results.filter((r) => r !== null);

  if (validResults.length === 0) {
    console.error("[FATAL] All agent pathways failed.");
    return;
  }

  let totalDamage = 0;
  validResults.forEach((res) => (totalDamage += res.Damage));
  const avgDamage = totalDamage / validResults.length;

  console.log("\n[SUCCESS] Benchmark Complete. Telemetry mapped:");

  // Formatted output logic
  const cps = validResults.length;
  const finalDps = (avgDamage * cps).toFixed(2);

  const tableData = validResults.map((r) => ({
    "Agent Node": r.Agent,
    "DPS = Damage × CPS = Result": `${r.Damage} × 1 = ${r.Damage}`,
    "Total Tokens": r.Tokens,
    "Latency (s)": r.Time,
  }));

  console.table(tableData);
  console.log(`\nSystem Throughput Formula:`);
  console.log(
    `DPS = Damage (${avgDamage.toFixed(2)} t/s) × CPS (${cps}) = Result (${finalDps} Total Tokens/sec)`,
  );
}

orchestrateBenchmark();
