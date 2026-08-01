"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { JSONPath } = require("jsonpath-plus");

const repoRoot = path.resolve(__dirname, "..", "..", "..");

test("release-please paths cover and transform every published version", () => {
  const config = JSON.parse(fs.readFileSync(path.join(repoRoot, "release-please-config.json"), "utf8"));
  const server = JSON.parse(fs.readFileSync(path.join(repoRoot, "server.json"), "utf8"));
  const paths = config.packages["."]["extra-files"]
    .filter((entry) => entry && entry.type === "json" && entry.path === "server.json")
    .map((entry) => entry.jsonpath);
  const next = "9.9.9";
  const versionPattern = /\d+\.\d+\.\d+(?:-[\w.]+)?(?:\+[-\w.]+)?/;

  for (const jsonpath of paths) {
    let matches = 0;
    JSONPath({
      resultType: "all",
      path: jsonpath,
      json: server,
      callback(payload) {
        matches += 1;
        assert.equal(typeof payload.value, "string", `${jsonpath} must select a string`);
        assert.match(payload.value, versionPattern, `${jsonpath} must select a versioned value`);
        payload.parent[payload.parentProperty] = payload.value.replace(versionPattern, next);
      },
    });
    assert.equal(matches, 1, `${jsonpath} must resolve exactly one live target`);
  }

  const publisher = server._meta["io.modelcontextprotocol.registry/publisher-provided"];
  assert.equal(server.version, next);
  assert.equal(server.packages.find((entry) => entry.identifier === "@dinglebear/rarcane").version, next);
  assert.equal(publisher.distribution.npm, `@dinglebear/rarcane@${next}`);
  assert.equal(publisher.buildInfo.version, next);
});

test("release-please paths transform both npm lockfile versions", () => {
  const config = JSON.parse(fs.readFileSync(path.join(repoRoot, "release-please-config.json"), "utf8"));
  const lockfile = JSON.parse(fs.readFileSync(path.join(repoRoot, "packages/arcane-rmcp/package-lock.json"), "utf8"));
  const paths = config.packages["."]["extra-files"]
    .filter((entry) => entry && entry.type === "json" && entry.path === "packages/arcane-rmcp/package-lock.json")
    .map((entry) => entry.jsonpath);
  const next = "9.9.9";

  for (const jsonpath of paths) {
    const matches = JSONPath({ resultType: "all", path: jsonpath, json: lockfile });
    assert.equal(matches.length, 1, `${jsonpath} must resolve exactly one lockfile target`);
    const [payload] = matches;
    payload.parent[payload.parentProperty] = next;
  }

  assert.equal(lockfile.version, next);
  assert.equal(lockfile.packages[""].version, next);
});
