"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  binaryVersion,
  downloadUrl,
  releaseBaseUrl,
  releaseVersion,
  targetFor,
} = require("../lib/platform");
const { version: packageVersion } = require("../package.json");

test("maps supported platforms to release assets", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "rarcane-x86_64.tar.gz",
    binary: "rarcane",
  });
  assert.deepEqual(targetFor("win32", "x64"), {
    asset: "rarcane-windows-x86_64.tar.gz",
    binary: "rarcane.exe",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("linux", "ppc64"), /Unsupported platform/);
});

test("uses the package version as the binary tag by default", () => {
  assert.equal(binaryVersion(), packageVersion);
  assert.equal(releaseVersion({}), `v${packageVersion}`);
});

test("allows release tag and repo overrides", () => {
  const env = { ARCANE_RMCP_BINARY_VERSION: "v9.9.9", ARCANE_RMCP_REPO: "example/arcane-rmcp" };
  assert.equal(releaseBaseUrl(env), "https://github.com/example/arcane-rmcp/releases/download");
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://github.com/example/arcane-rmcp/releases/download/v9.9.9/rarcane-x86_64.tar.gz");
});
