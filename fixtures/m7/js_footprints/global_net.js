module.exports = {
  params: {
    // Ensure we create one known net via params first.
    net: { type: 'net', value: 'GND' },
  },
  body: (p) => `
(segment (start 0 0) (end 1 0) (width 0.25) (layer "F.Cu") (net ${p.global_net('VCC')}))
`.trim(),
};

