# frozen_string_literal: true

# Based loosely on https://github.com/goreleaser/homebrew-tap/blob/master/Formula/goreleaser.rb.
# DO NOT ACTUALLY TAP THIS REPO! IT WILL BREAK IN THE FUTURE!
# This is committed here for testing purposes, so I don't lose it.

class YamlSplit < Formula
  desc "Split a YAML stream into individual documents"
  version "0.1.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      # TODO
    end

    if Hardware::CPU.intel?
      # TODO
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      # TODO
    end

    if Hardware::CPU.arm? && Hardware::CPU.is_64_bit?
      # TODO
    end

    if Hardware::CPU.arm? && !Hardware::CPU.is_64_bit?
      # TODO
    end
  end

  def install
    bin.install "yaml-split"
  end
end
