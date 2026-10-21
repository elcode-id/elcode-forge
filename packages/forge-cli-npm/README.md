# @elcode-forge/cli

Zero-install npm wrapper and binary runner for **ELCODE Forge (`eforge`)**.

```bash
# Instant zero-install execution via npx
npx @elcode-forge/cli doctor
npx @elcode-forge/cli scan
npx @elcode-forge/cli review --fast

# Or install globally
npm install -g @elcode-forge/cli
eforge doctor
```

## How It Works

1. Automatically resolves your OS and architecture (`win32-x64`, `linux-x64`, `darwin-arm64`).
2. Downloads official native release binaries directly from GitHub Releases (`v0.5.0`+).
3. Verifies binary integrity using official cryptographic `SHA256SUMS.txt`.
4. Caches the verified native executable locally (`~/.eforge/bin/` or `%LOCALAPPDATA%\eforge\bin\`).
5. Directly executes the native Rust binary with zero wrapper latency on subsequent runs.

## Environment Variables

- `EFORGE_BINARY_PATH`: Override native binary path with a local build.
- `EFORGE_CACHE_DIR`: Override destination cache directory for downloaded binaries.
- `EFORGE_DOWNLOAD_URL`: Override release assets download base URL (useful for air-gapped or internal proxy mirrors).

## License

MIT © [ELCODE](https://elcode.id/)
