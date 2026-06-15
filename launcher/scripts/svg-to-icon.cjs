// Rasterize an SVG to a 1024x1024 transparent PNG for `tauri icon`.
// Usage: node scripts/svg-to-icon.cjs <input.svg> <output.png>
const sharp = require("sharp");
const [, , input, output] = process.argv;

sharp(input, { density: 512 })
  .resize(1024, 1024, {
    fit: "contain",
    background: { r: 0, g: 0, b: 0, alpha: 0 },
  })
  .png()
  .toFile(output)
  .then(() => console.log("wrote", output))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
