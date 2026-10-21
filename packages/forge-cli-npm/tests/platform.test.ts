import { describe, it, expect } from 'vitest';
import path from 'node:path';
import os from 'node:os';
import fs from 'node:fs';
import crypto from 'node:crypto';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const {
  resolveTarget,
  getCacheDir,
  getBinaryPath,
  parseSha256Sums,
  verifyChecksum,
  runCli,
} = require('../src/index.js');

describe('@elcode-forge/cli — Platform & Resolution', () => {
  it('resolves supported platform and architecture targets', () => {
    expect(resolveTarget('win32', 'x64')).toEqual({
      target: 'x86_64-pc-windows-msvc',
      ext: 'zip',
      binaryName: 'eforge.exe',
    });

    expect(resolveTarget('linux', 'x64')).toEqual({
      target: 'x86_64-unknown-linux-gnu',
      ext: 'tar.gz',
      binaryName: 'eforge',
    });

    expect(resolveTarget('darwin', 'arm64')).toEqual({
      target: 'aarch64-apple-darwin',
      ext: 'tar.gz',
      binaryName: 'eforge',
    });
  });

  it('throws descriptive error on unsupported platforms or architectures', () => {
    expect(() => resolveTarget('freebsd', 'x64')).toThrow('Unsupported platform/architecture: freebsd-x64');
    expect(() => resolveTarget('win32', 'arm')).toThrow('Unsupported platform/architecture: win32-arm');
    expect(() => resolveTarget('linux', 'arm')).toThrow('Unsupported platform/architecture: linux-arm');
  });

  it('resolves cache directory respecting environment variables', () => {
    // Custom override
    expect(getCacheDir({ EFORGE_CACHE_DIR: '/custom/cache' })).toBe('/custom/cache');

    // Windows LOCALAPPDATA
    if (process.platform === 'win32') {
      const mockEnv = { LOCALAPPDATA: 'C:\\Users\\Mock\\AppData\\Local' };
      expect(getCacheDir(mockEnv)).toBe('C:\\Users\\Mock\\AppData\\Local\\eforge\\bin');
    }
  });

  it('resolves binary path and supports EFORGE_BINARY_PATH override', () => {
    const targetInfo = { binaryName: 'eforge.exe' };
    const customBin = '/opt/bin/eforge';
    expect(getBinaryPath('0.5.0', targetInfo, { EFORGE_BINARY_PATH: customBin })).toBe(customBin);

    const normalPath = getBinaryPath('0.5.0', targetInfo, { EFORGE_CACHE_DIR: '/cache' });
    expect(normalPath).toBe(path.join('/cache', 'v0.5.0', 'eforge.exe'));
  });

  it('parses SHA256SUMS.txt format deterministically', () => {
    const sample = `
# Release checksums
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  eforge-v0.5.0-x86_64-pc-windows-msvc.zip
f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2 *eforge-v0.5.0-x86_64-unknown-linux-gnu.tar.gz

01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b  eforge-v0.5.0-aarch64-apple-darwin.tar.gz
`;
    const map = parseSha256Sums(sample);
    expect(map.size).toBe(3);
    expect(map.get('eforge-v0.5.0-x86_64-pc-windows-msvc.zip')).toBe(
      'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
    );
    expect(map.get('eforge-v0.5.0-x86_64-unknown-linux-gnu.tar.gz')).toBe(
      'f2ca1bb6c7e907d06dafe4687e579fce76b37e4e93b7605022da52e6ccc26fd2'
    );
    expect(map.get('eforge-v0.5.0-aarch64-apple-darwin.tar.gz')).toBe(
      '01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b'
    );
  });

  it('verifies SHA256 file checksums accurately', () => {
    const tempFile = path.join(os.tmpdir(), `forge-test-${Date.now()}.txt`);
    const content = 'ELCODE Forge deterministic test payload';
    fs.writeFileSync(tempFile, content, 'utf8');

    const expectedHash = crypto.createHash('sha256').update(content).digest('hex');
    expect(verifyChecksum(tempFile, expectedHash)).toBe(true);
    expect(verifyChecksum(tempFile, '0000000000000000000000000000000000000000000000000000000000000000')).toBe(
      false
    );

    fs.unlinkSync(tempFile);
  });

  it('executes binary when EFORGE_BINARY_PATH is set', async () => {
    const repoRoot = path.resolve(__dirname, '../../..');
    const binaryExt = process.platform === 'win32' ? '.exe' : '';
    const localBinary = path.join(repoRoot, 'target', 'debug', `eforge${binaryExt}`);

    if (fs.existsSync(localBinary)) {
      const exitCode = await runCli(['--version'], {
        env: { ...process.env, EFORGE_BINARY_PATH: localBinary },
      });
      expect(exitCode).toBe(0);
    }
  });
});
