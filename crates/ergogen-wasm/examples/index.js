import init, { render_pcb } from "./pkg/ergogen_wasm.js";
import { installErgogenJsFootprints, registerErgogenJsFootprintSource } from "./footprints.js";

const yamlEl = document.getElementById("yaml");
const outputEl = document.getElementById("output");
const renderBtn = document.getElementById("render");

const demoYaml = `meta:
  author: Ergogen WASM
  version: v0.1
points.zones.matrix:
pcbs.pcb.template: kicad8
pcbs.pcb.footprints_search_paths:
  - .
pcbs.pcb.footprints:
  - what: simple
    params:
      net: GND
      label: JS
`;

const demoJs = `module.exports = {
  params: {
    net: { type: "net", value: "GND" },
    label: { type: "string", value: "JS" }
  },
  body: p => \`
(footprint "js_simple"
  (layer \${p.side}.Cu)
  \${p.at}
  (property "Reference" "\${p.ref}" (at 0 0) (layer \${p.side}.SilkS) \${p.ref_hide})
  (pad "1" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers *.Cu *.Mask) \${p.net})
)\`.trim()
};`;

yamlEl.value = demoYaml;

async function main() {
  await init();
  installErgogenJsFootprints();
  registerErgogenJsFootprintSource("simple.js", demoJs);
}

renderBtn.addEventListener("click", () => {
  try {
    const out = render_pcb(yamlEl.value, "pcb");
    outputEl.textContent = out;
  } catch (err) {
    outputEl.textContent = String(err);
  }
});

main();
