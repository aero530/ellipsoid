/**
 * Extract golden geometry data from the original JavaScript implementation.
 *
 * The Rust port in `crates/ellipsoid-core` is validated against these files
 * (see `tests/parity.rs`). Phase 1 of RUST_CONVERSION_PLAN.md.
 *
 *   node tools/extract-golden.mjs
 *
 * The JS is loaded from `app/utils/*.js` with three surgical patches applied to
 * in-memory copies, so nothing under `app/` is modified:
 *
 *   1. `lodash.clonedeep` -> `structuredClone`. The package is imported but is
 *      absent from package.json, and the cloned data is plain arrays of
 *      {x,y,z} numbers, for which the two are equivalent.
 *   2. `'./geometryHelpers'` -> `'./geometryHelpers.js'`, since ESM needs the
 *      extension.
 *   3. `import { settings } from 'paper'` is dropped. It is never used, and it
 *      would otherwise drag all of paper.js into a headless run.
 *
 * Together these let the reference run under plain `node` with no npm install,
 * which matters because a clean `yarn install` does not reproduce a working
 * tree (four imported packages are undeclared).
 *
 * `app/` NO LONGER EXISTS in the working tree — Phase 9 deleted it. The golden
 * files stand on their own, so this script is only needed if they have to be
 * regenerated. To do that, restore the reference from the tag that marks the
 * last commit containing it:
 *
 *   git checkout js-final -- app/utils
 *   node tools/extract-golden.mjs
 *   git rm -r --cached app && rm -rf app
 *
 * Node builtins only, so nothing needs installing first — which is just as
 * well, because package.json went with the rest of it.
 */

import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, '..');
const srcDir = join(repo, 'app', 'utils');
const workDir = join(repo, 'target', 'js-reference');
// Shared between ellipsoid-core (geometry, flattening, OBJ) and
// ellipsoid-pattern (2D layout), so it lives at the workspace root.
const outDir = join(repo, 'golden');

// --------------------------------------------------------------------------
// Patch and load the reference implementation
// --------------------------------------------------------------------------

function loadReference() {
  rmSync(workDir, { recursive: true, force: true });
  mkdirSync(workDir, { recursive: true });

  // The repo root package.json has no "type", so bare .js is treated as
  // CommonJS and the ESM `import` statements fail to parse. Mark this
  // directory as ESM so the sources load unmodified in that respect.
  writeFileSync(join(workDir, 'package.json'), '{ "type": "module" }\n');

  const helpers = readFileSync(join(srcDir, 'geometryHelpers.js'), 'utf8');
  writeFileSync(join(workDir, 'geometryHelpers.js'), helpers);

  let ellipsoid = readFileSync(join(srcDir, 'ellipsoid.js'), 'utf8');

  const patches = [
    [
      "import cloneDeep from 'lodash.clonedeep';",
      'const cloneDeep = structuredClone; // patched: see tools/extract-golden.mjs',
    ],
    ["} from './geometryHelpers';", "} from './geometryHelpers.js';"],
    ["import { settings } from 'paper';", '// patched out: unused paper.js import'],
  ];

  for (const [from, to] of patches) {
    if (!ellipsoid.includes(from)) {
      throw new Error(
        `Patch target not found in app/utils/ellipsoid.js:\n  ${from}\n` +
          'The reference implementation changed; update tools/extract-golden.mjs.',
      );
    }
    ellipsoid = ellipsoid.replace(from, to);
  }

  writeFileSync(join(workDir, 'ellipsoid.js'), ellipsoid);
  return import(pathToFileURL(join(workDir, 'ellipsoid.js')).href);
}

// --------------------------------------------------------------------------
// Parameter matrix
// --------------------------------------------------------------------------

// Field names are the originals (`Divisions` vs `divisions`) because these are
// fed straight into the untouched JS. The Rust side maps from EllipsoidInput.
const BASE = {
  a: 3.75,
  b: 2.875,
  c: 3.0,
  hTop: 0,
  hMiddle: 2,
  hBottom: 2,
  hTopFraction: 1.0,
  hTopShift: 0,
  Divisions: 8,
  divisions: 16,
  thetaMin: -35,
  thetaMax: 90,
  projection: 'cylindrical',
  // Used by drawEdges only.
  ppu: 96,
  minGap: 0.001,
  imageOffset: 0.5,
};

const cases = [];

/**
 * @param drawCapture record drawEdges output too. The 2D layout is a
 *   deterministic function of edgesFlat plus a handful of settings, so a
 *   curated subset covers it; capturing all 56 would quadruple fixture size
 *   for no extra coverage.
 */
const add = (name, overrides, drawCapture = false) =>
  cases.push({ name, settings: { ...BASE, ...overrides }, drawCapture });

// -- Theta sweep across both projections. These drive the sign-flip branches. --
for (const projection of ['spherical', 'cylindrical']) {
  for (const thetaMin of [-90, -35, 0, 30]) {
    for (const thetaMax of [90, 45, 0, -20]) {
      if (thetaMin >= thetaMax) continue;
      const tag = `${thetaMin}_${thetaMax}`.replace(/-/g, 'm');
      // Capture 2D layout for the default range and the fully-closed one.
      const draw = thetaMax === 90 && (thetaMin === -35 || thetaMin === -90);
      add(`theta_${projection}_${tag}`, { projection, thetaMin, thetaMax }, draw);
    }
  }
}

// -- The settings stamped on the sample output committed in screenshots/. --
// Useful as a reality check against a pattern a human actually produced.
add(
  'ref_screenshot_cylindrical',
  { projection: 'cylindrical', hMiddle: 2.625, hTopShift: 0.125 },
  true,
);

// -- Settings that only affect the 2D layout. --
for (const projection of ['spherical', 'cylindrical']) {
  add(`draw_mingap_large_${projection}`, { projection, minGap: 0.05 }, true);
  add(`draw_offset_zero_${projection}`, { projection, imageOffset: 0 }, true);
  add(`draw_units_mm_${projection}`, { projection, ppu: 3.7795276 }, true);
}

// -- Added-height combinations, the source of the special-cased rotations. --
for (const projection of ['spherical', 'cylindrical']) {
  const p = projection;
  add(`h_none_${p}`, { projection: p, hTop: 0, hMiddle: 0, hBottom: 0 }, true);
  // hTop > 0 takes a special branch in the outline assembly, so capture it.
  add(`h_top_${p}`, { projection: p, hTop: 1.5, hMiddle: 0, hBottom: 0, thetaMax: 45 }, true);
  add(`h_middle_${p}`, { projection: p, hTop: 0, hMiddle: 2, hBottom: 0 });
  add(`h_bottom_${p}`, { projection: p, hTop: 0, hMiddle: 0, hBottom: 1.25 });
  add(`h_all_${p}`, { projection: p, hTop: 1.5, hMiddle: 2, hBottom: 1.25, thetaMax: 45 });
  // hTop only matters when the top is open; pair the shaping knobs with it.
  add(`h_top_shaped_${p}`, {
    projection: p,
    hTop: 1.5,
    hTopFraction: 0.5,
    hTopShift: 0.75,
    thetaMax: 45,
  });
  add(`h_top_fraction_large_${p}`, {
    projection: p,
    hTop: 1.0,
    hTopFraction: 1.75,
    thetaMax: 60,
  });
}

// -- Division counts: parity of Divisions matters for the widest-point logic. --
for (const projection of ['spherical', 'cylindrical']) {
  add(`div_odd_${projection}`, { projection, Divisions: 7 });
  add(`div_even_${projection}`, { projection, Divisions: 6 });
  add(`div_min_${projection}`, { projection, Divisions: 3, divisions: 3 }, true);
  add(`div_min_theta_${projection}`, { projection, divisions: 3 });
  add(`div_asym_${projection}`, { projection, Divisions: 5, divisions: 11 });
}

// -- Degenerate / special shapes. --
for (const projection of ['spherical', 'cylindrical']) {
  add(`shape_sphere_${projection}`, { projection, a: 3, b: 3, c: 3 });
  add(`shape_oblate_${projection}`, { projection, a: 4, b: 4, c: 1.5 });
  add(`shape_prolate_${projection}`, { projection, a: 1.5, b: 1.5, c: 4 });
  add(`shape_closed_${projection}`, {
    projection,
    thetaMin: -90,
    thetaMax: 90,
    hMiddle: 0,
    hBottom: 0,
  });
}

// --------------------------------------------------------------------------
// A recording stand-in for paper.js
// --------------------------------------------------------------------------

/**
 * `drawEdges` only ever *writes* into the paper scope — it computes its own
 * bounds from `pattern.edgesFlat` and never reads geometry back. So a mock that
 * records what it was asked to draw captures its output exactly, with no
 * paper.js, no canvas, and no npm install.
 *
 * (`drawNotes` is different: it reads `bounds` off real paper items, so it
 * cannot be captured this way. See RUST_CONVERSION_PLAN.md §7.2.)
 */
function makeMockScope() {
  const layers = [];
  let active = null;

  class Point {
    constructor(x = 0, y = 0) {
      this.x = x;
      this.y = y;
    }
    set(x, y) {
      this.x = x;
      this.y = y;
    }
    add(other) {
      const [dx, dy] = Array.isArray(other) ? other : [other.x, other.y];
      return new Point(this.x + dx, this.y + dy);
    }
    divide(n) {
      return new Point(this.x / n, this.y / n);
    }
  }

  class Color {
    constructor(r, g, b, a) {
      this.rgba = [r, g, b, a === undefined ? 1 : a];
    }
  }

  class Item {
    constructor(kind) {
      this.kind = kind;
      this.owner = active;
      if (active) active.items.push(this);
    }
  }

  class Path extends Item {
    constructor(arg) {
      super('path');
      this.points = [];
      this.closed = false;
      if (typeof arg === 'string') {
        this.d = arg;
      } else if (arg && typeof arg === 'object') {
        Object.assign(this, arg);
      }
    }
    add(point) {
      this.points.push([point.x, point.y]);
    }
  }

  class Rectangle extends Item {
    constructor(from, to) {
      super('rect');
      this.from = [from.x, from.y];
      this.to = [to.x, to.y];
    }
  }

  class Group extends Item {
    constructor() {
      super('group');
      this.children = [];
    }
    addChild(item) {
      // Re-parent, mirroring paper: the item leaves the layer for the group.
      const siblings = item.owner ? item.owner.items : null;
      if (siblings) {
        const at = siblings.indexOf(item);
        if (at >= 0) siblings.splice(at, 1);
      }
      this.children.push(item);
    }
  }

  class Layer {
    constructor(name = null) {
      this.name = name;
      this.items = [];
      layers.push(this);
    }
    activate() {
      active = this;
    }
  }

  // scene.js names the initial layer before calling drawEdges.
  const patternLayer = new Layer('Ellipsoid Pattern');
  patternLayer.activate();

  const scope = {
    Point,
    Color,
    Path,
    Group,
    Layer,
    Shape: { Rectangle },
    project: {
      get activeLayer() {
        return active;
      },
    },
    _layers: layers,
  };

  return scope;
}

function serializeItem(item) {
  const out = { kind: item.kind };
  const color = (c) => (c instanceof Object && c.rgba ? c.rgba : c);

  if (item.kind === 'path') {
    if (item.d !== undefined) out.d = item.d;
    if (item.points.length) out.points = item.points;
    if (item.closed) out.closed = true;
  } else if (item.kind === 'rect') {
    out.from = item.from;
    out.to = item.to;
  } else if (item.kind === 'group') {
    out.children = item.children.map(serializeItem);
  }

  if (item.strokeColor !== undefined) out.strokeColor = color(item.strokeColor);
  if (item.strokeWidth !== undefined) out.strokeWidth = item.strokeWidth;
  if (item.fillColor !== undefined) out.fillColor = color(item.fillColor);
  return out;
}

function captureDraw(ref, settings, flat) {
  const scope = makeMockScope();
  ref.drawEdges(settings, flat, scope);
  return {
    layers: scope._layers.map((l) => ({
      name: l.name,
      items: l.items.map(serializeItem),
    })),
  };
}

// --------------------------------------------------------------------------
// Run
// --------------------------------------------------------------------------

/** Walk a structure and report the path of the first non-finite number. */
function findNonFinite(value, path = '') {
  if (typeof value === 'number') {
    return Number.isFinite(value) ? null : path || '(root)';
  }
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i += 1) {
      const hit = findNonFinite(value[i], `${path}[${i}]`);
      if (hit) return hit;
    }
    return null;
  }
  if (value && typeof value === 'object') {
    for (const [k, v] of Object.entries(value)) {
      const hit = findNonFinite(v, path ? `${path}.${k}` : k);
      if (hit) return hit;
    }
  }
  return null;
}

const ref = await loadReference();

// computeGeometry/computeFlatGeometry are extremely chatty on console.debug.
console.debug = () => {};

rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });

const written = [];
const skipped = [];

for (const { name, settings, drawCapture } of cases) {
  try {
    // computeGeometry mutates nothing, but pass a copy so a future change to
    // the reference cannot leak state between cases.
    const geometry = ref.computeGeometry({ ...settings });
    const flat = ref.computeFlatGeometry(geometry, { ...settings });

    const record = {
      name,
      settings,
      geometry: {
        Divisions: geometry.Divisions,
        divisions: geometry.divisions,
        indexWide: geometry.indexWide,
        points: geometry.geometry,
        obj: geometry.obj,
      },
      flat: {
        indexWide: flat.indexWide,
        edgesFlat: flat.edgesFlat,
        obj: flat.obj,
      },
    };

    const bad = findNonFinite({ points: record.geometry.points, edges: record.flat.edgesFlat });
    if (bad) {
      skipped.push({ name, reason: `non-finite value at ${bad}` });
      continue;
    }

    if (drawCapture) {
      record.draw = captureDraw(ref, { ...settings }, flat);
    }

    writeFileSync(join(outDir, `${name}.json`), `${JSON.stringify(record)}\n`);
    written.push(name);
  } catch (err) {
    skipped.push({ name, reason: `threw: ${err.message}` });
  }
}

writeFileSync(
  join(outDir, 'index.json'),
  `${JSON.stringify({ generated_by: 'tools/extract-golden.mjs', cases: written.sort(), skipped }, null, 2)}\n`,
);

console.log(`wrote ${written.length} golden cases to ${outDir}`);
if (skipped.length) {
  console.log(`\nskipped ${skipped.length}:`);
  for (const { name, reason } of skipped) console.log(`  ${name}: ${reason}`);
}
