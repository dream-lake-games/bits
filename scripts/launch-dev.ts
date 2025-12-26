import { spawn } from "bun";
import { CONFIG } from "./config";
import { positionWindow } from "./window-manager";

const COLORS = {
  blue: "\x1b[34m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  reset: "\x1b[0m",
} as const;

const processes: ReturnType<typeof spawn>[] = [];

function getRustLogEnv() {
  return `info,server=${CONFIG.logLevel},client=${CONFIG.logLevel},lightyear=${CONFIG.logLevel},bits=${CONFIG.logLevel},wgpu_core=warn,wgpu_hal=warn,naga=warn`;
}

function cleanupProcesses() {
  for (const proc of processes) {
    proc.kill();
  }
  process.exit(0);
}

process.on("SIGINT", cleanupProcesses);
process.on("SIGTERM", cleanupProcesses);

async function streamOutput(
  stream: ReadableStream,
  prefix: string,
  color: string,
  onLine?: (line: string) => void
) {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() || "";

    for (const line of lines) {
      if (line.trim()) {
        console.log(`${color}${prefix}${COLORS.reset} ${line}`);
        if (onLine) onLine(line);
      }
    }
  }
}

async function waitForServerReady(
  stdout: ReadableStream,
  stderr: ReadableStream
): Promise<void> {
  return new Promise((resolve) => {
    let ready = false;
    const timeout = setTimeout(() => {
      if (!ready) resolve();
    }, CONFIG.healthCheckTimeout);

    const checkLine = (line: string) => {
      if (line.includes(CONFIG.serverReadyMessage)) {
        ready = true;
        clearTimeout(timeout);
        resolve();
      }
    };

    streamOutput(stdout, "[SERVER]", COLORS.blue, checkLine);
    streamOutput(stderr, "[SERVER]", COLORS.blue, checkLine);
  });
}

async function launchServer() {
  console.log("🚀 Launching server...");

  const proc = spawn(["cargo", "run", "--bin", "server"], {
    env: { ...process.env, RUST_LOG: getRustLogEnv() },
    stdout: "pipe",
    stderr: "pipe",
    cwd: "..",
  });

  processes.push(proc);

  await waitForServerReady(proc.stdout, proc.stderr);
  await positionWindow("Server", CONFIG.server);

  console.log("✅ Server ready");
}

async function launchClient(
  name: string,
  config: { x: number; y: number },
  color: string
) {
  console.log(`🚀 Launching ${name}...`);

  const clientName = name.replace("CLIENT-", "");
  const windowTitle = `Client ${clientName}`;

  const proc = spawn(["cargo", "run", "--bin", "client"], {
    env: {
      ...process.env,
      RUST_LOG: getRustLogEnv(),
      CLIENT_NAME: clientName,
    },
    stdout: "pipe",
    stderr: "pipe",
    cwd: "..",
  });

  processes.push(proc);

  const windowReady = new Promise<void>((resolve) => {
    let ready = false;
    const timeout = setTimeout(() => {
      if (!ready) resolve();
    }, CONFIG.healthCheckTimeout);

    const checkLine = (line: string) => {
      if (line.includes(`Creating new window ${windowTitle}`)) {
        ready = true;
        clearTimeout(timeout);
        resolve();
      }
    };

    streamOutput(proc.stdout, `[${name}]`, color, checkLine);
    streamOutput(proc.stderr, `[${name}]`, color, checkLine);
  });

  await windowReady;
  await new Promise((resolve) =>
    setTimeout(resolve, CONFIG.windowPositionDelay)
  );
  await positionWindow(windowTitle, config);

  console.log(`✅ ${name} ready`);
}

async function main() {
  try {
    await launchServer();

    await Promise.all([
      launchClient("CLIENT-A", CONFIG.clientA, COLORS.green),
      launchClient("CLIENT-B", CONFIG.clientB, COLORS.yellow),
    ]);

    console.log("\n✨ All processes running. Press Ctrl+C to stop.\n");

    await new Promise(() => {});
  } catch (error) {
    console.error("Error:", error);
    cleanupProcesses();
  }
}

main();
