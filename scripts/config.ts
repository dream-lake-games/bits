interface Position {
  x: number;
  y: number;
}

interface Config {
  logLevel: string;
  server: Position;
  clientA: Position;
  clientB: Position;
  healthCheckTimeout: number;
  healthCheckInterval: number;
  serverReadyMessage: string;
  windowPositionDelay: number;
}

export const CONFIG: Config = {
  logLevel: "info",

  server: {
    x: 2320,
    // x: 0,
    y: 0,
  },

  clientA: {
    x: 2820,
    // x: 400,
    y: 0,
  },

  clientB: {
    x: 2820,
    // x: 400,
    y: 400,
  },

  healthCheckTimeout: 5000,
  healthCheckInterval: 100,
  serverReadyMessage: "Certificate hash:",
  windowPositionDelay: 100,
};
