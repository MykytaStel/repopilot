# Homebrew formula template for repopilot.
#
# To publish via Homebrew tap:
#   1. Create a new GitHub repo: MykytaStel/homebrew-repopilot
#   2. Copy this file to Formula/repopilot.rb in that repo
#   3. Replace each PLACEHOLDER_SHA256 with the real sha256 from the GitHub release
#      (found in repopilot-checksums.txt attached to the release)
#   4. Users can then install with:
#        brew tap mykytastel/repopilot
#        brew install repopilot

class Repopilot < Formula
  desc "Local-first CLI for repository audit and architecture risk detection"
  homepage "https://github.com/MykytaStel/repopilot"
  version "0.4.0"
  license "MIT OR Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MykytaStel/repopilot/releases/download/v#{version}/repopilot-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_intel do
      url "https://github.com/MykytaStel/repopilot/releases/download/v#{version}/repopilot-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/MykytaStel/repopilot/releases/download/v#{version}/repopilot-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_intel do
      url "https://github.com/MykytaStel/repopilot/releases/download/v#{version}/repopilot-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  def install
    bin.install "repopilot"
  end

  test do
    system "#{bin}/repopilot", "--version"
  end
end
