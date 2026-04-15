import { io } from "socket.io-client";

const socket = io("http://localhost:3001");

socket.on("connect", () => {
  console.log("Connected to dashboard server");

  const testPayload = {
    recentTasks: [
      {
        sessionId: "roxy:0001",
        taskName: "test-task",
        success: true,
        duration: 1500,
        timestamp: Date.now(),
      },
      {
        sessionId: "roxy:0002",
        taskName: "failed-task",
        success: false,
        duration: 2000,
        timestamp: Date.now(),
      },
    ],
  };

  console.log("Pushing test metrics...");
  socket.emit("push_metrics", testPayload);

  setTimeout(() => {
    console.log("Done.");
    process.exit(0);
  }, 1000);
});

socket.on("connect_error", (err) => {
  console.error("Connection error:", err.message);
  process.exit(1);
});
