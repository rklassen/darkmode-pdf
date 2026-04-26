"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const TARGETS = {
  "darwin-arm64": {
    rustTarget: "aarch64-apple-darwin",
    binaryName: "darkmode-pdf"
  },
  "win32-x64": {
    rustTarget: "x86_64-pc-windows-msvc",
    binaryName: "darkmode-pdf.exe"
  },
  "win32-arm64": {
    rustTarget: "aarch64-pc-windows-msvc",
    binaryName: "darkmode-pdf.exe"
  },
  "linux-x64": {
    rustTarget: "x86_64-unknown-linux-gnu",
    binaryName: "darkmode-pdf"
  },
  "linux-arm64": {
    rustTarget: "aarch64-unknown-linux-gnu",
    binaryName: "darkmode-pdf"
  }
};

function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help || !args.target) {
    printUsage(args.help ? 0 : 1);
    return;
  }

  const target = TARGETS[args.target];
  if (!target) {
    fail(
      `Unsupported target \`${args.target}\`. Expected one of: ${Object.keys(TARGETS).join(", ")}`
    );
  }

  const scriptDir = __dirname;
  const extensionRoot = path.resolve(scriptDir, "..");
  const repoRoot = path.resolve(extensionRoot, "..");
  const cargoManifestPath = path.join(repoRoot, "Cargo.toml");

  if (!fs.existsSync(cargoManifestPath)) {
    fail(`Cargo manifest not found at \`${cargoManifestPath}\`.`);
  }

  if (args.build) {
    buildTarget(repoRoot, cargoManifestPath, target.rustTarget);
  }

  const sourceBinaryPath = resolveSourceBinaryPath(args, repoRoot, target);
  if (!fs.existsSync(sourceBinaryPath)) {
    fail(
      `Source binary not found at \`${sourceBinaryPath}\`. Pass --build or provide --binary /path/to/${target.binaryName}.`
    );
  }

  const destinationDir = path.join(extensionRoot, "bin", args.target);
  const destinationBinaryPath = path.join(destinationDir, target.binaryName);

  fs.mkdirSync(destinationDir, { recursive: true });
  fs.copyFileSync(sourceBinaryPath, destinationBinaryPath);

  if (target.binaryName.endsWith(".exe")) {
    console.log(`Staged ${args.target}: ${destinationBinaryPath}`);
    return;
  }

  fs.chmodSync(destinationBinaryPath, 0o755);
  console.log(`Staged ${args.target}: ${destinationBinaryPath}`);
}

function buildTarget(repoRoot, cargoManifestPath, rustTarget) {
  const cargoArgs = [
    "build",
    "--release",
    "--target",
    rustTarget,
    "--manifest-path",
    cargoManifestPath
  ];
  const result = spawnSync("cargo", cargoArgs, {
    cwd: repoRoot,
    stdio: "inherit"
  });

  if (result.status !== 0) {
    fail(`Cargo build failed for Rust target \`${rustTarget}\`.`);
  }
}

function resolveSourceBinaryPath(args, repoRoot, target) {
  if (args.binary) {
    return path.resolve(process.cwd(), args.binary);
  }

  return path.join(
    repoRoot,
    "target",
    target.rustTarget,
    "release",
    target.binaryName
  );
}

function parseArgs(argv) {
  const args = {
    target: "",
    binary: "",
    build: false,
    help: false
  };

  for (let idx = 0; idx < argv.length; idx += 1) {
    const value = argv[idx];

    if (value === "--build") {
      args.build = true;
      continue;
    }
    if (value === "--help" || value === "-h") {
      args.help = true;
      continue;
    }
    if (value === "--target") {
      args.target = argv[idx + 1] || "";
      idx += 1;
      continue;
    }
    if (value === "--binary") {
      args.binary = argv[idx + 1] || "";
      idx += 1;
      continue;
    }

    fail(`Unknown argument: ${value}`);
  }

  return args;
}

function printUsage(exitCode) {
  const lines = [
    "Usage:",
    "  node ./scripts/stage-binary.js --target <vscode-target> [--build]",
    "  node ./scripts/stage-binary.js --target <vscode-target> --binary <path>",
    "",
    "Examples:",
    "  node ./scripts/stage-binary.js --target darwin-arm64 --build",
    "  node ./scripts/stage-binary.js --target win32-arm64 --binary ../artifacts/darkmode-pdf.exe"
  ];
  const writer = exitCode === 0 ? console.log : console.error;
  writer(lines.join("\n"));
  process.exit(exitCode);
}

function fail(message) {
  console.error(message);
  process.exit(1);
}

main();
