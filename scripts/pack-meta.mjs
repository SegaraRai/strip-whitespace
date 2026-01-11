import fs from "node:fs/promises";
import path from "node:path";

const GENERATED_MANIFEST = ".pack-meta.generated.json";

function parseArgs(argv) {
  return {
    cleanup: argv.includes("--cleanup"),
  };
}

async function fileExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function writeGeneratedManifest(packageDir, files) {
  const manifestPath = path.join(packageDir, GENERATED_MANIFEST);
  await fs.writeFile(
    manifestPath,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        files,
      },
      null,
      2,
    ) + "\n",
    "utf8",
  );
}

async function readGeneratedManifest(packageDir) {
  const manifestPath = path.join(packageDir, GENERATED_MANIFEST);
  const raw = await fs.readFile(manifestPath, "utf8");
  return JSON.parse(raw);
}

async function main() {
  const { cleanup } = parseArgs(process.argv.slice(2));
  const packageDir = process.cwd();
  const repoRoot = path.resolve(packageDir, "..", "..");

  const manifestPath = path.join(packageDir, GENERATED_MANIFEST);

  if (cleanup) {
    if (!(await fileExists(manifestPath))) return;

    const manifest = await readGeneratedManifest(packageDir);
    await Promise.all(
      (manifest.files ?? []).map(async (relativePath) => {
        const target = path.join(packageDir, relativePath);
        if (await fileExists(target)) {
          await fs.rm(target);
        }
      }),
    );

    await fs.rm(manifestPath);
    return;
  }

  const rootLicense = path.join(repoRoot, "LICENSE");

  const generatedFiles = [];

  const licenseTarget = path.join(packageDir, "LICENSE");
  if (!(await fileExists(licenseTarget))) {
    await fs.copyFile(rootLicense, licenseTarget);
    generatedFiles.push("LICENSE");
  }

  if (generatedFiles.length > 0) {
    await writeGeneratedManifest(packageDir, generatedFiles);
  }
}

await main();
