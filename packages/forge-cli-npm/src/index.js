/**
 * elcode-forge — Zero-install launcher and native binary manager for ELCODE Forge (`eforge`).
 *
 * Implements deterministic binary downloading, local caching, SHA256 verification, and process spawning
 * with zero external npm runtime dependencies.
 */

const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const crypto = require('node:crypto');
const https = require('node:https');
const child_process = require('node:child_process');

const REPO_OWNER = 'elcode-id';
const REPO_NAME = 'elcode-forge';

/**
 * Resolves the platform and architecture to release target metadata.
 *
 * @param {string} [platform=process.platform]
 * @param {string} [arch=process.arch]
 * @returns {{ target: string, ext: string, binaryName: string }}
 */
function resolveTarget(platform = process.platform, arch = process.arch) {
  if (platform === 'win32' && arch === 'x64') {
    return {
      target: 'x86_64-pc-windows-msvc',
      ext: 'zip',
      binaryName: 'eforge.exe',
    };
  }

  if (platform === 'linux' && arch === 'x64') {
    return {
      target: 'x86_64-unknown-linux-gnu',
      ext: 'tar.gz',
      binaryName: 'eforge',
    };
  }

  if (platform === 'darwin' && arch === 'arm64') {
    return {
      target: 'aarch64-apple-darwin',
      ext: 'tar.gz',
      binaryName: 'eforge',
    };
  }

  throw new Error(
    `Unsupported platform/architecture: ${platform}-${arch}. ` +
      `Supported platforms: win32-x64, linux-x64, darwin-arm64. ` +
      `You can build from source using 'cargo install --path crates/forge-cli'.`
  );
}

/**
 * Resolves the cache directory for downloaded binaries.
 *
 * @param {NodeJS.ProcessEnv} [env=process.env]
 * @returns {string}
 */
function getCacheDir(env = process.env) {
  if (env.EFORGE_CACHE_DIR) {
    return env.EFORGE_CACHE_DIR;
  }

  if (process.platform === 'win32') {
    if (env.LOCALAPPDATA) {
      return path.join(env.LOCALAPPDATA, 'eforge', 'bin');
    }
    return path.join(os.homedir(), '.eforge', 'bin');
  }

  return path.join(os.homedir(), '.eforge', 'bin');
}

/**
 * Resolves the absolute path to the cached or custom binary.
 *
 * @param {string} version
 * @param {{ binaryName: string }} targetInfo
 * @param {NodeJS.ProcessEnv} [env=process.env]
 * @returns {string}
 */
function getBinaryPath(version, targetInfo, env = process.env) {
  if (env.EFORGE_BINARY_PATH) {
    return env.EFORGE_BINARY_PATH;
  }

  return path.join(getCacheDir(env), `v${version.replace(/^v/, '')}`, targetInfo.binaryName);
}

/**
 * Parses a SHA256SUMS.txt file into a Map of filename -> sha256 hash.
 *
 * @param {string} content
 * @returns {Map<string, string>}
 */
function parseSha256Sums(content) {
  const map = new Map();
  const lines = content.split(/\r?\n/);

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) {
      continue;
    }

    const match = trimmed.match(/^([a-fA-F0-9]{64})\s+[*]?(.+)$/);
    if (match) {
      const hash = match[1].toLowerCase();
      const filename = path.basename(match[2].trim());
      map.set(filename, hash);
    }
  }

  return map;
}

/**
 * Computes and verifies the SHA-256 hash of a local file against an expected hash.
 *
 * @param {string} filePath
 * @param {string} expectedHash
 * @returns {boolean}
 */
function verifyChecksum(filePath, expectedHash) {
  const data = fs.readFileSync(filePath);
  const hash = crypto.createHash('sha256').update(data).digest('hex').toLowerCase();
  return hash === expectedHash.toLowerCase();
}

/**
 * Downloads a URL to a local destination file following redirects.
 *
 * @param {string} url
 * @param {string} destPath
 * @param {number} [maxRedirects=5]
 * @returns {Promise<void>}
 */
function downloadFile(url, destPath, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    if (maxRedirects < 0) {
      return reject(new Error(`Too many redirects downloading ${url}`));
    }

    https
      .get(
        url,
        {
          headers: {
            'User-Agent': 'elcode-forge-launcher',
          },
        },
        (res) => {
          if (
            res.statusCode &&
            [301, 302, 307, 308].includes(res.statusCode) &&
            res.headers.location
          ) {
            const redirectUrl = res.headers.location.startsWith('http')
              ? res.headers.location
              : new URL(res.headers.location, url).toString();
            return resolve(downloadFile(redirectUrl, destPath, maxRedirects - 1));
          }

          if (res.statusCode !== 200) {
            return reject(
              new Error(`Failed to download ${url}: HTTP status code ${res.statusCode}`)
            );
          }

          const fileStream = fs.createWriteStream(destPath);
          res.pipe(fileStream);

          fileStream.on('finish', () => {
            fileStream.close();
            resolve();
          });

          fileStream.on('error', (err) => {
            fs.unlink(destPath, () => {});
            reject(err);
          });
        }
      )
      .on('error', reject);
  });
}

/**
 * Fetches content of a URL into a string in memory following redirects.
 *
 * @param {string} url
 * @param {number} [maxRedirects=5]
 * @returns {Promise<string>}
 */
function fetchText(url, maxRedirects = 5) {
  return new Promise((resolve, reject) => {
    if (maxRedirects < 0) {
      return reject(new Error(`Too many redirects fetching ${url}`));
    }

    https
      .get(
        url,
        {
          headers: {
            'User-Agent': 'elcode-forge-launcher',
          },
        },
        (res) => {
          if (
            res.statusCode &&
            [301, 302, 307, 308].includes(res.statusCode) &&
            res.headers.location
          ) {
            const redirectUrl = res.headers.location.startsWith('http')
              ? res.headers.location
              : new URL(res.headers.location, url).toString();
            return resolve(fetchText(redirectUrl, maxRedirects - 1));
          }

          if (res.statusCode !== 200) {
            return reject(new Error(`Failed to fetch ${url}: HTTP status code ${res.statusCode}`));
          }

          let data = '';
          res.setEncoding('utf8');
          res.on('data', (chunk) => (data += chunk));
          res.on('end', () => resolve(data));
        }
      )
      .on('error', reject);
  });
}

/**
 * Extracts a release archive using native OS tooling without external dependencies.
 *
 * @param {string} archivePath
 * @param {string} extractDir
 * @param {string} ext
 */
function extractArchive(archivePath, extractDir, ext) {
  fs.mkdirSync(extractDir, { recursive: true });

  if (process.platform === 'win32') {
    // Attempt standard tar.exe first (shipped in Windows 10 build 1803+ and Windows 11)
    try {
      child_process.execFileSync('tar.exe', ['-xf', archivePath, '-C', extractDir], {
        stdio: 'ignore',
      });
      return;
    } catch {
      // Fallback to PowerShell Expand-Archive
      child_process.execFileSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          `Expand-Archive -Path "${archivePath}" -DestinationPath "${extractDir}" -Force`,
        ],
        { stdio: 'ignore' }
      );
      return;
    }
  }

  // Unix tar
  child_process.execFileSync('tar', ['-xzf', archivePath, '-C', extractDir], {
    stdio: 'ignore',
  });
}

/**
 * Locates the target binary inside an extracted directory (which may contain a subfolder)
 * and moves it to the target binary location.
 *
 * @param {string} extractDir
 * @param {string} targetBinaryPath
 * @param {string} binaryName
 */
function findAndPlaceBinary(extractDir, targetBinaryPath, binaryName) {
  // Check directly in extractDir
  const directPath = path.join(extractDir, binaryName);
  if (fs.existsSync(directPath)) {
    fs.renameSync(directPath, targetBinaryPath);
    return;
  }

  // Check subdirectories
  const entries = fs.readdirSync(extractDir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      const nestedPath = path.join(extractDir, entry.name, binaryName);
      if (fs.existsSync(nestedPath)) {
        fs.renameSync(nestedPath, targetBinaryPath);
        return;
      }
    }
  }

  throw new Error(`Binary '${binaryName}' was not found inside the extracted archive.`);
}

/**
 * Ensures that the native binary is downloaded, verified, and ready in the local cache.
 *
 * @param {string} version
 * @param {object} [opts={}]
 * @param {string} [opts.platform]
 * @param {string} [opts.arch]
 * @param {NodeJS.ProcessEnv} [opts.env]
 * @returns {Promise<string>} Path to the executable binary
 */
async function ensureBinary(version, opts = {}) {
  const env = opts.env || process.env;
  const targetInfo = resolveTarget(opts.platform, opts.arch);
  const binaryPath = getBinaryPath(version, targetInfo, env);

  // If already cached and valid, return immediately
  if (fs.existsSync(binaryPath)) {
    if (process.platform !== 'win32') {
      try {
        fs.chmodSync(binaryPath, 0o755);
      } catch {}
    }
    return binaryPath;
  }

  const cleanVersion = version.replace(/^v/, '');
  const versionTag = `v${cleanVersion}`;
  const archiveName = `eforge-${versionTag}-${targetInfo.target}.${targetInfo.ext}`;
  const baseUrl =
    env.EFORGE_DOWNLOAD_URL ||
    `https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${versionTag}`;

  const downloadArchiveUrl = `${baseUrl}/${archiveName}`;
  const sha256Url = `${baseUrl}/SHA256SUMS.txt`;

  const cacheDir = path.dirname(binaryPath);
  fs.mkdirSync(cacheDir, { recursive: true });

  const tempArchive = path.join(cacheDir, `temp-${archiveName}`);
  const tempExtractDir = path.join(cacheDir, `temp-extract-${Date.now()}`);

  try {
    process.stderr.write(
      `[eforge] Downloading ELCODE Forge ${versionTag} for ${targetInfo.target}...\n`
    );

    // 1. Fetch SHA256SUMS.txt
    const sumsContent = await fetchText(sha256Url);
    const sumsMap = parseSha256Sums(sumsContent);
    const expectedHash = sumsMap.get(archiveName);

    if (!expectedHash) {
      throw new Error(`Checksum for '${archiveName}' not found in SHA256SUMS.txt.`);
    }

    // 2. Download binary archive
    await downloadFile(downloadArchiveUrl, tempArchive);

    // 3. Verify SHA256 checksum
    const isValid = verifyChecksum(tempArchive, expectedHash);
    if (!isValid) {
      throw new Error(
        `SHA256 checksum verification failed for '${archiveName}'. Integrity check rejected.`
      );
    }

    // 4. Extract archive
    extractArchive(tempArchive, tempExtractDir, targetInfo.ext);

    // 5. Place binary into destination
    findAndPlaceBinary(tempExtractDir, binaryPath, targetInfo.binaryName);

    // 6. Ensure executable permissions on Unix
    if (process.platform !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    process.stderr.write(`[eforge] Installed native binary to ${binaryPath}\n`);
    return binaryPath;
  } finally {
    // Cleanup temporary files
    try {
      if (fs.existsSync(tempArchive)) {
        fs.unlinkSync(tempArchive);
      }
      if (fs.existsSync(tempExtractDir)) {
        fs.rmSync(tempExtractDir, { recursive: true, force: true });
      }
    } catch {}
  }
}

/**
 * Runs the native `eforge` binary forwarding all arguments and inheriting stdio.
 *
 * @param {string[]} [argv=process.argv.slice(2)]
 * @param {object} [opts={}]
 * @returns {Promise<number>} Exit code of the native binary process
 */
async function runCli(argv = process.argv.slice(2), opts = {}) {
  const pkgJson = require('../package.json');
  const binaryPath = await ensureBinary(pkgJson.version, opts);

  const res = child_process.spawnSync(binaryPath, argv, {
    stdio: 'inherit',
    env: process.env,
  });

  if (res.error) {
    throw res.error;
  }

  return res.status ?? 0;
}

module.exports = {
  resolveTarget,
  getCacheDir,
  getBinaryPath,
  parseSha256Sums,
  verifyChecksum,
  downloadFile,
  fetchText,
  extractArchive,
  ensureBinary,
  runCli,
};
