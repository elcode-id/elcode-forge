<svelte:head>
  <title>SLSA Level 3 & Cryptographic Provenance | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-emerald-400 mb-1">Supply-Chain Security</div>
    <h1 class="text-3xl font-extrabold text-white">SLSA Level 3 Build Provenance</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Verify the cryptographic authenticity and tamper-proof build provenance of official ELCODE Forge release binaries.
    </p>
  </div>

  <!-- Verification Methods -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. GitHub Artifact Attestation Verification</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      All release binaries and tarballs are signed using GitHub Artifact Attestations backed by Sigstore Fulcio and Rekor.
    </p>
    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs text-emerald-400">
      gh attestation verify eforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz --owner elcode-id
    </div>
  </div>

  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">2. OpenSSF SLSA-Verifier</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      For strict enterprise compliance, verify the signed in-toto statement using OpenSSF <code class="text-blue-300 font-mono">slsa-verifier</code>:
    </p>
    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-300 overflow-x-auto">
      <pre><code>slsa-verifier verify-artifact eforge-v1.0.0-x86_64-unknown-linux-gnu.tar.gz \
  --provenance-path multiple.intoto.jsonl \
  --source-uri github.com/elcode-id/elcode-forge \
  --source-tag v1.0.0</code></pre>
    </div>
  </div>
</div>
