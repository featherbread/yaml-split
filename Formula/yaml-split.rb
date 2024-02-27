# frozen_string_literal: true

# Based loosely on https://github.com/goreleaser/homebrew-tap/blob/master/Formula/goreleaser.rb.
# DO NOT ACTUALLY TAP THIS REPO! IT WILL BREAK IN THE FUTURE!
# This is committed here for testing purposes, so I don't lose it.

class YamlSplit < Formula
  desc "Split a YAML stream into individual documents"
  version "0.1.5"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.5/yaml-split-aarch64-apple-darwin.tar.gz"
      sha256 "21f069c276ed6bbcb69c4f72df43b34d8b15bfd6c84d765294d50d2c13091e93"
    end

    if Hardware::CPU.intel?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.5/yaml-split-x86_64-apple-darwin.tar.gz"
      sha256 "e04f0291888490279612cf13e8efaca4d7d71240a461f7f8a3c348e432d00e9e"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.5/yaml-split-x86_64-unknown-linux-musl.tar.gz"
      sha256 "61e14ed0da4c0ab2e01a05e77c17889961476672686682a76c31b788514d3015"
    end

    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.5/yaml-split-aarch64-unknown-linux-musl.tar.gz"
      sha256 "607395123054b28f9c885a53bfb722e87e6efe116dfa684aebdfb8121c6dd229"
    end

    if Hardware::CPU.arm? && !Hardware::CPU.is_64_bit?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.5/yaml-split-armv7-unknown-linux-musleabihf.tar.gz"
      sha256 "9f7244d4b730d8a1a777212c71b2d456cb439fa55cf92a67e8d8a998e65863c0"
    end
  end

  def install
    bin.install "yaml-split"
  end
end
