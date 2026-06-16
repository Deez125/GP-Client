// Rename the bundled NSIS installer from "GP Client_x.x.x_x64-setup.exe" to
// "GP_Client_x.x.x_x64-setup.exe" (underscore). We keep the app's productName
// as "GP Client" (so the install folder / shortcut / auto-update target stay
// stable); only the distributed installer file is renamed.
const fs = require("fs");
const path = require("path");

const dir = path.join(
  __dirname,
  "..",
  "src-tauri",
  "target",
  "release",
  "bundle",
  "nsis",
);

if (!fs.existsSync(dir)) {
  console.log(`rename-installer: no nsis bundle dir at ${dir}, skipping.`);
  process.exit(0);
}

let renamed = 0;
for (const file of fs.readdirSync(dir)) {
  if (file.startsWith("GP Client") && file.toLowerCase().endsWith(".exe")) {
    const dest = file.replace(/^GP Client/, "GP_Client");
    fs.renameSync(path.join(dir, file), path.join(dir, dest));
    console.log(`rename-installer: ${file} -> ${dest}`);
    renamed++;
  }
}
if (renamed === 0) {
  console.log("rename-installer: nothing to rename.");
}
