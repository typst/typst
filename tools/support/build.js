const fs = require("fs");
const process = require("process");

process.chdir(__dirname);

const file = fs.readFileSync("theme.json");
const repository = JSON.parse(file);

function emit(mode) {
  const filename = `typst-${mode}.tmLanguage.json`;
  const theme = {
    name: "typst-" + mode,
    scopeName: "source.typst-" + mode,
    patterns: [{ include: "#" + mode }],
    repository,
  };
  fs.writeFileSync(filename, JSON.stringify(theme, null, 2), "utf8");
}

emit("markup");
emit("code");
