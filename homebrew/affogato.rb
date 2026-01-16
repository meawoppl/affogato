# Homebrew formula for Affogato
# Install: brew tap meawoppl/tools && brew install affogato
# Or: brew install meawoppl/tools/affogato

class Affogato < Formula
  desc "ESP32-S2 + ICE40 FPGA development tool"
  homepage "https://github.com/meawoppl/affogato"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/meawoppl/affogato/releases/download/v#{version}/affogato-x86_64-apple-darwin.tar.gz"
      # sha256 "UPDATE_SHA256_HERE"
    end
    on_arm do
      url "https://github.com/meawoppl/affogato/releases/download/v#{version}/affogato-aarch64-apple-darwin.tar.gz"
      # sha256 "UPDATE_SHA256_HERE"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/meawoppl/affogato/releases/download/v#{version}/affogato-x86_64-unknown-linux-gnu.tar.gz"
      # sha256 "UPDATE_SHA256_HERE"
    end
    on_arm do
      url "https://github.com/meawoppl/affogato/releases/download/v#{version}/affogato-aarch64-unknown-linux-gnu.tar.gz"
      # sha256 "UPDATE_SHA256_HERE"
    end
  end

  depends_on "docker" => :recommended

  def install
    bin.install "affogato"
  end

  test do
    system "#{bin}/affogato", "--version"
  end
end
