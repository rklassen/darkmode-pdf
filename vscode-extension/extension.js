"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { execFile } = require("node:child_process");
const { promisify } = require("node:util");
const vscode = require("vscode");

const execFileAsync = promisify(execFile);
const EXTENSION_ID = "darkmode-pdf";
const SUPPORTED_LINUX_IDS = new Set(["ubuntu", "arch"]);

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "darkmodePdf.renderCurrentFile",
      runCommand(async (resource) => {
        await renderCurrentFile(context.extensionPath, resource);
      })
    ),
    vscode.commands.registerCommand(
      "darkmodePdf.showRuntimeInfo",
      runCommand(async () => {
        const runtime = await detectRuntime(context.extensionPath);
        const lines = [
          `platform: ${runtime.platform}`,
          `arch: ${runtime.arch}`,
          `target id: ${runtime.targetId}`,
          `linux distro: ${runtime.linuxId || "n/a"}`,
          `binary path: ${runtime.binaryPath || "not resolved"}`,
          `remote session: ${vscode.env.remoteName || "no"}`
        ];
        void vscode.window.showInformationMessage(lines.join(" | "));
      })
    )
  );
}

async function renderCurrentFile(extensionPath, resource) {
  if (vscode.env.remoteName) {
    throw new Error(
      "Darkmode PDF is scaffolded as a local desktop extension and does not support remote extension hosts."
    );
  }

  const inputUri = await resolveMarkdownUri(resource);
  await ensureSaved(inputUri);
  const runtime = await detectRuntime(extensionPath);

  const outputUri = await vscode.window.showSaveDialog({
    defaultUri: inputUri.with({ path: replaceExtname(inputUri.fsPath, ".pdf") }),
    filters: {
      PDF: ["pdf"]
    },
    saveLabel: "Render PDF"
  });

  if (!outputUri) {
    return;
  }

  await ensureExecutable(runtime.binaryPath);

  const renderTask = execFileAsync(runtime.binaryPath, [inputUri.fsPath, outputUri.fsPath], {
    cwd: extensionPath,
    windowsHide: true,
    maxBuffer: 16 * 1024 * 1024
  });

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Rendering dark-mode PDF",
      cancellable: false
    },
    async () => {
      try {
        const { stderr } = await renderTask;
        if (stderr && stderr.trim()) {
          console.warn(`[${EXTENSION_ID}] renderer stderr: ${stderr}`);
        }
      } catch (error) {
        throw new Error(formatExecError(error));
      }
    }
  );

  const openAction = "Open PDF";
  const revealAction = "Reveal in File Manager";
  const choice = await vscode.window.showInformationMessage(
    `Rendered ${path.basename(outputUri.fsPath)}.`,
    openAction,
    revealAction
  );

  if (choice === openAction) {
    await vscode.env.openExternal(outputUri);
  } else if (choice === revealAction) {
    await vscode.commands.executeCommand("revealFileInOS", outputUri);
  }
}

function runCommand(handler) {
  return async (...args) => {
    try {
      await handler(...args);
    } catch (error) {
      void vscode.window.showErrorMessage(formatUnknownError(error));
    }
  };
}

async function resolveMarkdownUri(resource) {
  if (resource instanceof vscode.Uri) {
    return validateMarkdownUri(resource);
  }

  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    throw new Error("Open a Markdown file before running Darkmode PDF.");
  }

  return validateMarkdownUri(editor.document.uri);
}

async function ensureSaved(uri) {
  const document = vscode.workspace.textDocuments.find(
    (item) => item.uri.scheme === uri.scheme && item.uri.fsPath === uri.fsPath
  );
  if (!document || !document.isDirty) {
    return;
  }
  const saved = await document.save();
  if (!saved) {
    throw new Error("Save the Markdown file before rendering the PDF.");
  }
}

function validateMarkdownUri(uri) {
  if (uri.scheme !== "file") {
    throw new Error("Darkmode PDF only supports local files.");
  }
  if (path.extname(uri.fsPath).toLowerCase() !== ".md") {
    throw new Error("Darkmode PDF only renders `.md` files.");
  }
  return uri;
}

async function detectRuntime(extensionPath) {
  const platform = process.platform;
  const arch = process.arch;
  const linuxId = platform === "linux" ? await detectLinuxId() : null;
  const targetId = getTargetId(platform, arch, linuxId);
  const binaryName = platform === "win32" ? "darkmode-pdf.exe" : "darkmode-pdf";
  const configuredBinaryPath = getConfiguredBinaryPath();
  const binaryPath =
    configuredBinaryPath ||
    path.join(extensionPath, "bin", targetId, binaryName);

  if (!fs.existsSync(binaryPath)) {
    throw new Error(
      `No renderer binary was found for target \`${targetId}\` at \`${binaryPath}\`. Build or copy the platform binary into the extension package first.`
    );
  }

  return {
    platform,
    arch,
    linuxId,
    targetId,
    binaryPath
  };
}

function getConfiguredBinaryPath() {
  const value = vscode.workspace
    .getConfiguration("darkmodePdf")
    .get("binaryPath", "")
    .trim();
  return value || "";
}

function getTargetId(platform, arch, linuxId) {
  if (platform === "darwin") {
    if (arch !== "arm64") {
      throw new Error("This extension scaffold only supports Apple Silicon on macOS (`darwin-arm64`).");
    }
    return "darwin-arm64";
  }

  if (platform === "win32") {
    if (arch === "arm64") {
      return "win32-arm64";
    }
    if (arch === "x64") {
      return "win32-x64";
    }
    throw new Error(`Unsupported Windows architecture: ${arch}.`);
  }

  if (platform === "linux") {
    if (!linuxId) {
      throw new Error("Unable to identify the current Linux distribution from `/etc/os-release`.");
    }
    if (!SUPPORTED_LINUX_IDS.has(linuxId)) {
      throw new Error(
        `Unsupported Linux distribution: ${linuxId}. This scaffold is intentionally limited to Ubuntu and Arch.`
      );
    }
    if (arch !== "x64" && arch !== "arm64") {
      throw new Error(`Unsupported Linux architecture: ${arch}.`);
    }
    return `linux-${arch}`;
  }

  throw new Error(`Unsupported platform: ${platform}.`);
}

async function detectLinuxId() {
  try {
    const osReleasePath = "/etc/os-release";
    const raw = await fs.promises.readFile(osReleasePath, "utf8");
    for (const line of raw.split(/\r?\n/)) {
      if (!line.startsWith("ID=")) {
        continue;
      }
      return line.slice(3).trim().replace(/^"/, "").replace(/"$/, "").toLowerCase();
    }
  } catch (error) {
    console.warn(`[${EXTENSION_ID}] failed to read os-release: ${formatUnknownError(error)}`);
  }

  return null;
}

async function ensureExecutable(binaryPath) {
  if (process.platform === "win32") {
    return;
  }
  await fs.promises.chmod(binaryPath, 0o755);
}

function replaceExtname(filePath, ext) {
  return path.join(path.dirname(filePath), `${path.basename(filePath, path.extname(filePath))}${ext}`);
}

function formatExecError(error) {
  if (!error || typeof error !== "object") {
    return String(error);
  }

  const pieces = [];
  if (error.message) {
    pieces.push(error.message);
  }
  if (typeof error.stdout === "string" && error.stdout.trim()) {
    pieces.push(`stdout: ${error.stdout.trim()}`);
  }
  if (typeof error.stderr === "string" && error.stderr.trim()) {
    pieces.push(`stderr: ${error.stderr.trim()}`);
  }
  return pieces.join("\n");
}

function formatUnknownError(error) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
