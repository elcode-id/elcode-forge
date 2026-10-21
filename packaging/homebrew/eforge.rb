class Eforge < Formula
  desc "Know your code. Ship with confidence. Instant architectural intelligence for codebases"
  homepage "https://elcode.id/"
  version "0.6.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/elcode-id/elcode-forge/releases/download/v#{version}/eforge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "afb1629cb8cc2966cdd5495132ab87af4ab3599aeb0d17b1b39276d13487eb2e"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/elcode-id/elcode-forge/releases/download/v#{version}/eforge-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "31f2fd68cdb1c8941b51690b34bb1e1df58a8bee13aeab04132c7f8ebcd38ec7"
    end
  end

  def install
    bin.install "eforge"
    bin.install "elcode-forge" if File.exist?("elcode-forge")
  end

  test do
    assert_match "ELCODE Forge", shell_output("#{bin}/eforge --version")
    assert_match "ELCODE Forge", shell_output("#{bin}/elcode-forge --version")
  end
end
