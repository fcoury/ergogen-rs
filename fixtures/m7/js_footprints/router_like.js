module.exports = {
  params: {
    net: { type: "net", value: "GND" },
    width: { type: "number", value: 0.25 },
    route: { type: "string", value: "" },
    locked: { type: "boolean", value: false },
    via_size: { type: "number", value: 0.8 },
    via_drill: { type: "number", value: 0.4 }
  },

  body: (p) => {
    const pattern = /\(at (-?[\d.]+) (-?[\d.]+) (-?[\d.]+)\)/;
    const matches = p.at.match(pattern);
    if (!matches) throw new Error("failed to parse p.at: " + p.at);

    const atX = parseFloat(matches[1]);
    const atY = parseFloat(matches[2]);
    const atAngle = parseFloat(matches[3]);

    const radians = (Math.PI / 180) * atAngle;
    const cos = Math.cos(radians);
    const sin = Math.sin(radians);
    const adjustPoint = (x, y) => {
      const nx = cos * x + sin * y + atX;
      const ny = cos * y - sin * x + atY;
      return `${nx.toFixed(5) / 1} ${ny.toFixed(5) / 1}`;
    };

    const parseTuple = (t) => {
      const strTuple = JSON.parse(t.replace(/\(/g, "[").replace(/\)/g, "]"));
      const numTuple = strTuple.map((v) => Number(v));
      if (isNaN(numTuple[0]) || isNaN(numTuple[1])) {
        throw new Error("invalid position: " + t);
      }
      return numTuple;
    };

    const locked = p.locked ? "locked " : "";
    let layer = undefined;
    let start = undefined;
    let net = p.net.index;

    let out = "ERGOGEN_ROUTER_LIKE\n";
    const route = p.route || "";
    for (let i = 0; i < route.length; i++) {
      const ch = route[i].toLowerCase();
      switch (ch) {
        case "f":
          layer = "\"F.Cu\"";
          break;
        case "b":
          layer = "\"B.Cu\"";
          break;
        case "v":
          if (!start) break;
          out += `(via ${locked}(at ${adjustPoint(start[0], start[1])}) (size ${
            p.via_size
          }) (drill ${p.via_drill}) (layers "F.Cu" "B.Cu") (net ${net}))\n`;
          break;
        case "(":
          {
            let tupleStr = "(";
            for (i = i + 1; i < route.length; i++) {
              tupleStr += route[i];
              if (route[i] === ")") break;
            }
            const pos = parseTuple(tupleStr);
            if (start) {
              if (!layer) throw new Error("missing layer (use f/b)");
              out += `(segment ${locked}(start ${adjustPoint(
                start[0],
                start[1]
              )}) (end ${adjustPoint(pos[0], pos[1])}) (width ${
                p.width
              }) (layer ${layer}) (net ${net}))\n`;
            }
            start = pos;
          }
          break;
        case "<":
          {
            let netName = "";
            for (i = i + 1; i < route.length; i++) {
              if (route[i] === ">") break;
              netName += route[i];
            }
            net = p.global_net(netName);
            start = undefined;
          }
          break;
        case "x":
        case "|":
          start = undefined;
          break;
        default:
          break;
      }
    }

    return out.trim();
  }
};

