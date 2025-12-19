function loadModule(source) {
  const module = { exports: {} };
  const exports = module.exports;
  const fn = new Function("module", "exports", source);
  fn(module, exports);
  return module.exports || {};
}

function lookupSource(path) {
  const sources = globalThis.__ergogenFootprintSources;
  if (!sources) return null;
  if (Object.prototype.hasOwnProperty.call(sources, path)) return sources[path];
  const base = String(path).split(/[\\/]/).pop();
  if (base && Object.prototype.hasOwnProperty.call(sources, base)) {
    return sources[base];
  }
  return null;
}

export function registerErgogenJsFootprintSource(path, source) {
  if (!globalThis.__ergogenFootprintSources) {
    globalThis.__ergogenFootprintSources = {};
  }
  globalThis.__ergogenFootprintSources[path] = source;
  const base = String(path).split(/[\\/]/).pop();
  if (base) {
    globalThis.__ergogenFootprintSources[base] = source;
  }
}

export function ergogenJsFootprintParams(source) {
  const mod = loadModule(source);
  return mod.params || {};
}

export function ergogenRenderJsFootprint(source, p) {
  const mod = loadModule(source);
  if (typeof mod.body !== "function") {
    throw new Error("JS footprint module.exports.body must be a function");
  }
  return String(mod.body(p));
}

export function ergogenLoadJsFootprintSource(path) {
  const fromMap = lookupSource(path);
  if (fromMap !== null) {
    return String(fromMap);
  }
  if (typeof globalThis.ergogenJsFootprintSourceLoader === "function") {
    return String(globalThis.ergogenJsFootprintSourceLoader(path));
  }
  if (typeof require === "function") {
    // Node fallback for tests.
    const fs = require("fs");
    return fs.readFileSync(path, "utf8");
  }
  throw new Error(`No JS footprint source loader for ${path}`);
}

export function installErgogenJsFootprints() {
  globalThis.ergogenJsFootprintParams = ergogenJsFootprintParams;
  globalThis.ergogenRenderJsFootprint = ergogenRenderJsFootprint;
  globalThis.ergogenLoadJsFootprintSource = ergogenLoadJsFootprintSource;
}
