const { booleans, extrusions, primitives, transforms, measurements } = require('@jscad/modeling');
const { union, subtract, intersect } = booleans;
const { extrudeLinear } = extrusions;
const { rectangle, circle } = primitives;
const { translate, rotate } = transforms;
const { measureBoundingBox } = measurements;

function export_extrude_1_outline_fn(){
  const shape = rectangle({ size: [18, 18], center: [0, 0] });
  return extrudeLinear({ height: 1 }, shape);
}


function export_case_fn() {

  // creating part 0 of case export
  let export__part_0 = export_extrude_1_outline_fn();

  // make sure that rotations are relative
  let export__part_0_bounds = measureBoundingBox(export__part_0);
  let export__part_0_x = export__part_0_bounds[0][0] + (export__part_0_bounds[1][0] - export__part_0_bounds[0][0]) / 2;
  let export__part_0_y = export__part_0_bounds[0][1] + (export__part_0_bounds[1][1] - export__part_0_bounds[0][1]) / 2;
  export__part_0 = translate([-export__part_0_x, -export__part_0_y, 0], export__part_0);
  export__part_0 = rotate([0,0,0], export__part_0);
  export__part_0 = translate([export__part_0_x, export__part_0_y, 0], export__part_0);

  export__part_0 = translate([0,0,0], export__part_0);
  let result = export__part_0;

  return result;
}

const main = () => export_case_fn();
module.exports = { main };
