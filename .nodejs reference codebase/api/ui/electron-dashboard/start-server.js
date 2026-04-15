import { DashboardServer } from "./server/index.js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const CONFIG_FILE = path.join(__dirname, "config.json");

// Load config to get default port
let defaultPort = 3001;
try {
  if (fs.existsSync(CONFIG_FILE)) {
    const config = JSON.parse(fs.readFileSync(CONFIG_FILE, "utf8"));
    defaultPort = config?.server?.port || 3001;
  }
} catch (err) {
  console.warn("[start-server] Could not load config, using default port 3001");
}

// Allow environment variable override
const port = parseInt(process.env.PORT) || defaultPort;
const server = new DashboardServer(port);
server.start().catch((err) => {
  console.error("Failed to start server:", err);
  process.exit(1);
});
