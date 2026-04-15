import { app, BrowserWindow, ipcMain } from "electron";
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { startStandaloneServer } from "./server/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

app.commandLine.appendSwitch("disable-gpu");
app.commandLine.appendSwitch("disable-software-rasterizer");

let mainWindow;
let dashboardConfig = {};
let dashboardServer = null;
let weStartedServer = false;

function getWindowConfigPath() {
  return path.join(app.getPath("userData"), "window-config.json");
}

function loadDashboardConfig() {
  try {
    const configPath = path.join(__dirname, "config.json");
    if (fs.existsSync(configPath)) {
      const data = fs.readFileSync(configPath, "utf8");
      dashboardConfig = JSON.parse(data);
      console.log("[Dashboard] Config loaded:", dashboardConfig);
    }
  } catch (err) {
    console.error("[Dashboard] Failed to load config:", err.message);
  }
  return dashboardConfig;
}

function loadWindowBounds() {
  try {
    const configPath = getWindowConfigPath();
    if (fs.existsSync(configPath)) {
      const data = fs.readFileSync(configPath, "utf8");
      return JSON.parse(data);
    }
  } catch (err) {
    console.error("Failed to load window config:", err);
  }
  return null;
}

function saveWindowBounds() {
  if (!mainWindow) return;
  try {
    const bounds = mainWindow.getBounds();
    const isMaximized = mainWindow.isMaximized();
    const configPath = getWindowConfigPath();
    fs.writeFileSync(configPath, JSON.stringify({ bounds, isMaximized }));
  } catch (err) {
    console.error("Failed to save window config:", err);
  }
}

async function createWindow() {
  const config = loadDashboardConfig();
  const windowConfig = loadWindowBounds();

  const serverPort = config?.server?.port || 3001;
  const serverHost = config?.server?.host || "localhost";
  const hasRemoteHost =
    config?.connect?.remoteHost && config.connect.remoteHost.trim().length > 0;
  const connectToRemote = hasRemoteHost;

  let serverStarted = false;

  if (!connectToRemote) {
    // Start the dashboard server standalone (unless connecting to remote)
    try {
      dashboardServer = await startStandaloneServer(serverPort, 5000);
      weStartedServer = true;
      console.log(`[Dashboard] Server started on port ${serverPort}`);
    } catch (err) {
      if (err.code === "EADDRINUSE") {
        console.log(
          `[Dashboard] Port ${serverPort} already in use - connecting to existing server`,
        );
        weStartedServer = false;
      } else {
        console.error("[Dashboard] Failed to start server:", err.message);
      }
    }
  } else {
    console.log(
      `[Dashboard] Remote mode: connecting to ${config.connect.remoteHost}:${config.connect.remotePort}`,
    );
  }

  const serverUrl = connectToRemote
    ? `http://${config.connect.remoteHost}:${config.connect.remotePort}`
    : `http://${serverHost}:${serverPort}`;

  const windowOptions = {
    width:
      windowConfig?.bounds?.width || (config?.ui?.defaultCompact ? 400 : 1200),
    height:
      windowConfig?.bounds?.height || (config?.ui?.defaultCompact ? 600 : 800),
    x: windowConfig?.bounds?.x,
    y: windowConfig?.bounds?.y,
    minWidth: 600,
    minHeight: 400,
    icon: path.join(__dirname, "icon.ico"),
    backgroundColor: "#0d0d14",
    autoHideMenuBar: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, "preload.mjs"),
    },
    resizable: true,
    title: "Auto-AI Dashboard",
    show: false,
  };

  mainWindow = new BrowserWindow(windowOptions);

  // Apply default always on top
  if (config?.ui?.defaultAlwaysOnTop) {
    mainWindow.setAlwaysOnTop(true);
  }

  if (windowConfig?.isMaximized) {
    mainWindow.maximize();
  }

  // Pass server URL to renderer via query params
  const serverParam = encodeURIComponent(serverUrl);
  const devServerPort = dashboardConfig?.devServer?.port || 5173;

  // Load the app
  if (!app.isPackaged) {
    // Dev mode: Load from Vite dev server
    mainWindow
      .loadURL(`http://localhost:${devServerPort}?server=${serverParam}`)
      .catch(() => {
        console.log(
          `[Dashboard] Dev server not found at :${devServerPort}, falling back to dist`,
        );
        const reactPath = path.join(
          __dirname,
          "renderer",
          "dist",
          "index.html",
        );
        mainWindow.loadFile(reactPath, { query: { server: serverUrl } });
      });
  } else {
    // Production: Load React build directly
    const reactPath = path.join(__dirname, "renderer", "dist", "index.html");
    mainWindow
      .loadFile(reactPath, { query: { server: serverUrl } })
      .catch((err) => {
        console.error("Failed to load React app:", err);
        mainWindow.loadURL(
          "data:text/html;charset=utf-8," +
            encodeURIComponent(`
                    <html><body style="background:#000;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;">
                        <div style="text-align:center;">
                            <h1>Dashboard Build Missing</h1>
                            <p>Run: <code>cd renderer && npm run build</code></p>
                            <p style="margin-top:20px;font-size:12px;color:#888;">Or start server only: <code>node start-server.js</code></p>
                        </div>
                    </body></html>
                `),
        );
      });
  }

  mainWindow.once("ready-to-show", () => {
    mainWindow.show();
  });

  mainWindow.on("close", () => {
    saveWindowBounds(mainWindow);
  });

  // Stop dashboard server when window closes
  mainWindow.on("closed", async () => {
    const serverToStop = dashboardServer;
    mainWindow = null;
    dashboardServer = null;
    if (serverToStop && weStartedServer) {
      console.log("[Dashboard] Stopping server...");
      try {
        await serverToStop.stop();
        console.log("[Dashboard] Server stopped");
      } catch (err) {
        console.error("[Dashboard] Error stopping server:", err.message);
      }
    }
  });

  // IPC Handlers
  ipcMain.handle("get-config", () => {
    return {
      serverUrl: serverUrl,
      port: serverPort,
      ...dashboardConfig,
    };
  });

  ipcMain.on("toggle-always-on-top", (event) => {
    const isAlwaysOnTop = mainWindow.isAlwaysOnTop();
    mainWindow.setAlwaysOnTop(!isAlwaysOnTop);
    console.log(`[Dashboard] Always on top: ${!isAlwaysOnTop}`);
  });

  ipcMain.on("set-window-size", (event, { width, height, compact }) => {
    // Validate width and height to prevent malicious values
    if (typeof width !== "number" || width < 100 || width > 5000) {
      console.error(`[Dashboard] Invalid window width: ${width}`);
      return;
    }
    if (typeof height !== "number" || height < 100 || height > 5000) {
      console.error(`[Dashboard] Invalid window height: ${height}`);
      return;
    }
    mainWindow.setSize(width, height);
    // If compact, we might want to disable some UI elements via IPC if needed
    // but for now, just resizing is enough.
    console.log(
      `[Dashboard] Window resized to: ${width}x${height} (Compact: ${compact})`,
    );
  });
}

app.whenReady().then(createWindow);

// Force stop server on quit - covers all quit scenarios
app.on("before-quit", async () => {
  if (dashboardServer && weStartedServer) {
    console.log("[Dashboard] Force stopping server on quit...");
    try {
      await dashboardServer.stop();
      console.log("[Dashboard] Server stopped");
    } catch (err) {
      console.error("[Dashboard] Error stopping server:", err.message);
    }
  }
});

app.on("window-all-closed", async () => {
  app.quit();
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

process.on("uncaughtException", (error) => {
  console.error("Uncaught Exception in Electron main process:", error);
  app.quit();
});

process.on("unhandledRejection", (reason, promise) => {
  console.error(
    "Unhandled Promise Rejection in Electron main process:",
    reason,
  );
  app.quit();
});
