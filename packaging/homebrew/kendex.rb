# Homebrew formula for the kendex CLI. Lives in the tap
# `vanillagreencom/homebrew-kendex`, installed with:
#
#   brew install vanillagreencom/kendex/kendex
#
# Stable installs the prebuilt release binary; `--HEAD` builds from source.
class Kendex < Formula
  desc "Package manager for agents, skills, and hooks across AI coding tools"
  homepage "https://kendex.ai"
  version "5.0.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/vanillagreencom/kendex/releases/download/v#{version}/kendex-aarch64-apple-darwin"
      sha256 "e1a1d7199afc8ce08e7c9cb19ccc313f4489c3b256aa787de9310de7a3816c87"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/vanillagreencom/kendex/releases/download/v#{version}/kendex-x86_64-unknown-linux-gnu"
      sha256 "a3dee4c286614016198db72603fcf95de277ddf1a245da052dc815821f0e84c0"
    end
  end

  head "https://github.com/vanillagreencom/kendex.git", branch: "main"

  depends_on "rust" => :build if build.head?

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args(path: "crates/cli"), "--bin", "kendex"
    else
      bin.install Dir["*"].first => "kendex"
    end
  end

  test do
    assert_match "5.0.1", shell_output("#{bin}/kendex --version")
  end
end
