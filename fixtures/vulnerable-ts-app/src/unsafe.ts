import { exec } from 'child_process';

// Synthetic test credential patterns for deterministic secret detection tests (NOT real keys)
export const SYNTHETIC_OPENAI_KEY = 'sk-proj-1234567890abcdefghijklmnopQRSTUVWX';
export const SYNTHETIC_GITHUB_PAT = 'ghp_1234567890abcdefghijklmnopqrstuvwxyz';

export function runDynamicCode(userInput: string): unknown {
  // Unsafe eval usage
  return eval(userInput);
}

export function runUnsafeShell(cmd: string): void {
  // Unsafe shell execution
  exec(`sh -c "${cmd}"`);
}

export const INSECURE_CORS_HEADER = {
  'Access-Control-Allow-Origin': '*',
};
