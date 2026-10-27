import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, existsSync, readFileSync } from "node:fs";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dirname, "..");
const testRoot = join(root, "test-data");
mkdirSync(testRoot, { recursive: true });
const data = mkdtempSync(join(testRoot, "e2e-"));
const executable = process.platform === "win32" ? "prm.exe" : "prm";
const environment = { ...process.env, PRM_PORT: "4318", PRM_DATA_DIR: data };
delete environment.PRM_AGENT_TOKEN;
delete environment.PRM_DATABASE_SCHEMA; // Each temporary data directory gets an isolated schema.
const pgConfig = join(root, ".tools", "postgresql.json");
if (!environment.PRM_DATABASE_URL && existsSync(pgConfig)) environment.PRM_DATABASE_URL = JSON.parse(readFileSync(pgConfig, "utf8").replace(/^\uFEFF/, "")).url;
if (!environment.PRM_DATABASE_URL) throw new Error("Set PRM_DATABASE_URL for PostgreSQL integration tests");
const server = spawn(join(root, "target", "debug", executable), [], {
  cwd: root,
  windowsHide: true,
  stdio: "inherit",
  env: environment,
});
server.on("error", (error) => {
  console.error(error);
  process.exit(1);
});
server.on("exit", (code) => process.exit(code || 0));
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => {
    server.kill();
  });
process.on("exit", () => {
  server.kill();
});
