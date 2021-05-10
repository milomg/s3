const { build } = require("esbuild");
const fs = require("fs-extra");
const chokidar = require("chokidar");

let prod = process.argv[2] == "production";
function runBuild() {
  build({
    entryPoints: ["./src/index.ts"],
    outfile: "dist/main.js",
    sourcemap: true,
    bundle: true,
    minify: prod,
  }).then(() => {
    console.log("Wrote to dist/main.js");
  });
}

function copy() {
  if (!fs.existsSync("dist")) fs.mkdirSync("dist");
  fs.copy("assets", "dist", (err) => {
    if (err) throw err;
    console.log("Copied assets to dist");
  });
}

if (!prod) {
  chokidar.watch("src").on("change", runBuild);
  chokidar.watch("assets").on("change", copy);
}

runBuild();
copy();
