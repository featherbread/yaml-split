# frozen_string_literal: true

# Based loosely on https://github.com/goreleaser/homebrew-tap/blob/master/Formula/goreleaser.rb.
# DO NOT ACTUALLY TAP THIS REPO! IT WILL BREAK IN THE FUTURE!
# This is committed here for testing purposes, so I don't lose it.

class YamlSplit < Formula
  desc "Split a YAML stream into individual documents"
  version "0.1.8"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.8/yaml-split-aarch64-apple-darwin.tar.gz"
      sha256 "717768bba25389b2ed275e7bb163a7772b5545584f2950a135efb784352c4c1f"
    end

    if Hardware::CPU.intel?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.8/yaml-split-x86_64-apple-darwin.tar.gz"
      sha256 "5e059569a82bf00da0abeddf981018ae7f0fec10e2f48b6f34721550ad9e904b"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.8/yaml-split-x86_64-unknown-linux-musl.tar.gz"
      sha256 "8b767adbe555445dc5f9d9611814c8931c9c7a275cbe9d1179fd2d7e6f90d110"
    end

    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.8/yaml-split-aarch64-unknown-linux-musl.tar.gz"
      sha256 "0e52a3ff1c3eeb9393fca805910043bdc3e003922664df4469f637d892636c19"
    end

    if Hardware::CPU.arm? && !Hardware::CPU.is_64_bit?
      url "https://github.com/ahamlinman/yaml-split/releases/download/v0.1.8/yaml-split-armv7-unknown-linux-musleabihf.tar.gz"
      sha256 "9b736233f76dd4b234c0889804754753aef57e7c3ab3897b1e1f64265e8cfd29"
    end
  end

  def install
    bin.install "yaml-split-*/yaml-split"
    man1.install "yaml-split-*/yaml-split.1"
    doc.install "yaml-split-*/LICENSES.html"
  end
end
