class Eforge < Formula
  desc "Know your code. Ship with confidence. Instant architectural intelligence for codebases"
  homepage "https://elcode.id/"
  version "1.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/elcode-id/elcode-forge/releases/download/v#{version}/eforge-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "90d556c4aa31a40065a585a7687a86b6611fd0ab9bf9a3427d4b46a6527b60ec"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/elcode-id/elcode-forge/releases/download/v#{version}/eforge-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e9b25c5ca2666fc8107af390e5fa64e3790b784b867b2bcbaa57ffd8332155e1"
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
