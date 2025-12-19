module.exports = {
  params: {
    net: { type: "net", value: "GND" },
    label: { type: "string", value: "JS" }
  },
  body: p => `
(footprint "js_simple"
  (layer ${p.side}.Cu)
  ${p.at}
  (property "Reference" "${p.ref}" (at 0 0) (layer ${p.side}.SilkS) ${p.ref_hide})
  (pad "1" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers *.Cu *.Mask) ${p.net})
  (pad "2" thru_hole circle (at ${p.xy(1, 2)}) (size 1 1) (drill 0.5) (layers *.Cu *.Mask) ${p.local_net("A")})
  (fp_text user "${p.label}" (at 0 1) (layer ${p.side}.SilkS))
)
`.trim()
};
