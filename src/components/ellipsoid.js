//import * as vis from 'vis';

import cloneDeep from 'lodash.clonedeep';
import { angleBetweenPlanes, pointToString, rotatePoint, distance} from './geometryHelpers';

// http://paulbourke.net/geometry/ellipsecirc/
// http://pages.pacificcoast.net/~cazelais/250a/ellipse-length.pdf
// https://www.mathworks.com/matlabcentral/fileexchange/52958-intersection-ellipsoid-and-a-plane?s_tid=gn_loc_drop

// x = a cos(theta) cos(phi)
// y = b cos(theta) sin(phi)
// z = c sin(theta)
// -pi/2 <= theta <= pi/2
// =pi <= phi <= pi

// http://seiyria.com/bootstrap-slider/


function panelExtents(array) {
  const arrayMax = {
    x: Number.NEGATIVE_INFINITY,
    y: Number.NEGATIVE_INFINITY,
    z: Number.NEGATIVE_INFINITY
  };
  const arrayMin = {
    x: Number.POSITIVE_INFINITY,
    y: Number.POSITIVE_INFINITY,
    z: Number.POSITIVE_INFINITY
  };

  for (let indexp = 0; indexp < array.length; indexp++) {
    for (let indext = 0; indext < array[indexp].length; indext++) {
      if (array[indexp][indext][0].x > arrayMax.x) {
        arrayMax.x = array[indexp][indext][0].x;
      }
      if (array[indexp][indext][1].x > arrayMax.x) {
        arrayMax.x = array[indexp][indext][1].x;
      }

      if (array[indexp][indext][0].y > arrayMax.y) {
        arrayMax.y = array[indexp][indext][0].y;
      }
      if (array[indexp][indext][1].y > arrayMax.y) {
        arrayMax.y = array[indexp][indext][1].y;
      }

      if (array[indexp][indext][0].z > arrayMax.z) {
        arrayMax.z = array[indexp][indext][0].z;
      }
      if (array[indexp][indext][1].z > arrayMax.z) {
        arrayMax.z = array[indexp][indext][1].z;
      }

      if (array[indexp][indext][0].x < arrayMin.x) {
        arrayMin.x = array[indexp][indext][0].x;
      }
      if (array[indexp][indext][1].x < arrayMin.x) {
        arrayMin.x = array[indexp][indext][1].x;
      }

      if (array[indexp][indext][0].y < arrayMin.y) {
        arrayMin.y = array[indexp][indext][0].y;
      }
      if (array[indexp][indext][1].y < arrayMin.y) {
        arrayMin.y = array[indexp][indext][1].y;
      }

      if (array[indexp][indext][0].z < arrayMin.z) {
        arrayMin.z = array[indexp][indext][0].z;
      }
      if (array[indexp][indext][1].z < arrayMin.z) {
        arrayMin.z = array[indexp][indext][1].z;
      }
    }
  }
  return [arrayMin, arrayMax];
}

// -----------------------------------------------------------------------------
// Drawing functions
// -----------------------------------------------------------------------------

// function drawVisualization(panels, elementid) {
//   // Function to generate 3D plot of panels

//   // create the data table.
//   data = new vis.DataSet();

//   // track the max value in each axis. used to scale the z axis

//   var maxX = 0;
//   var maxZ = 0;
//   var minZ = 0;

//   // create the animation data
//   $.each(panels, function(idx0, slice) {
//     $.each(slice, function(idx1, line) {
//       if (Math.abs(line[0].x) > maxX) {
//         maxX = Math.abs(line[0].x);
//       } // abs value is used because if the number of phi divisions is odd the "max" extent may be on the negative side
//       if (line[0].z > maxZ) {
//         maxZ = line[0].z;
//       }
//       if (line[0].z < minZ) {
//         minZ = line[0].z;
//       }
//       if (Math.abs(line[1].x) > maxX) {
//         maxX = Math.abs(line[1].x);
//       }
//       if (line[1].z > maxZ) {
//         maxZ = line[1].z;
//       }
//       if (line[1].z < minZ) {
//         minZ = line[1].z;
//       }
//       //minZ = (line[1].z < minZ) ? line[1].z : minZ;
//       data.add({
//         x: line[0].x,
//         y: line[0].y,
//         z: line[0].z,
//         style: parseInt(idx0) // color the dots by the panel column they are in
//       });
//       data.add({
//         x: line[1].x,
//         y: line[1].y,
//         z: line[1].z,
//         style: parseInt(idx0)
//       });
//     });
//   });

//   // specify options
//   const options = {
//     width: "100%",
//     height: "600px",
//     style: "dot-color",
//     showPerspective: false,
//     showGrid: true,
//     keepAspectRatio: true,
//     verticalRatio: (maxZ - minZ) / maxX * 0.5,
//     legendLabel: "value",
//     cameraPosition: {
//       horizontal: -0.25,
//       vertical: 0.25,
//       distance: 1.6
//     }
//   };

//   // create our graph
//   const container = document.getElementById(elementid);
//   graph = new vis.Graph3d(container, data, options);
// }

export function computePattern(state) {
  // Reference equations for an ellipsoid
  // x = a cos(theta) cos(phi)
  // y = b cos(theta) sin(phi)
  // z = c sin(theta)
  // -pi/2 <= theta <= pi/2 (use 0 to pi/2 to get the top half)
  // -pi <= phi <= pi
  const a = parseFloat(state.a);
  const b = parseFloat(state.b);
  const c = parseFloat(state.c);
  const htop = parseFloat(state.hTop);
  const hmiddle = parseFloat(state.hMiddle);
  const hbottom = parseFloat(state.hBottom);
  const htopfraction = parseFloat(state.hTopFraction);
  const htopshift = parseFloat(state.hTopShift);
  let Divisions = parseFloat(state.Divisions);
  let divisions = parseFloat(state.divisions);
  const theta_min = parseFloat(state.thetaMin) * Math.PI / 180;
  const theta_max = parseFloat(state.thetaMax) * Math.PI / 180;
  const projection = state.projection;


  // -----------------------------------------------------------------------------
  // Compute basic dimensions of the ellipsoid
  // -----------------------------------------------------------------------------

  // -----------------------------------------------------------------------------
  // create array of theta and phi angles
  // -----------------------------------------------------------------------------

  const thetas = Array.apply(null, Array(divisions + 1)).map(function(_, i) {
    return theta_min + i * ((theta_max - theta_min) / divisions);
  });

  // Enforce that if thetas spans 0 that 0 is included in the array
  if (theta_max > 0 && theta_min < 0) {
    let idx = 0;
    while (thetas[idx] < 0) {
      idx++;
    }
    if (thetas[idx] !== 0) {
      // if theta=0 is not already in the array
      thetas.splice(idx, 0, 0);
      divisions = divisions + 1;
    }
  }

  const phis = Array.apply(null, Array(Divisions + 1)).map(function(_, i) {
    return -Math.PI + i * (2 * Math.PI / Divisions);
  });

  console.log("thetas");
  console.log(thetas);
  console.log("phis");
  console.log(phis);

  // --------------------------------------------------------------------------
  // generate all the points that make up the ellipsoid approximation based on the number of divisions specified
  // --------------------------------------------------------------------------
  var ellipsoid = [];
  for (let indexp = 0; indexp <= Divisions; indexp++) {
    let phi = phis[indexp];
    ellipsoid[indexp] = [];
    for (let indext = 0; indext <= divisions; indext++) {
      let theta = thetas[indext];
      ellipsoid[indexp][indext] = {
        x: a * Math.cos(theta) * Math.cos(phi),
        y: b * Math.cos(theta) * Math.sin(phi),
        z: c * Math.sin(theta)
      };
    }
  }

  // --------------------------------------------------------------------------
  // Add height to ellipsoid
  // --------------------------------------------------------------------------

  if (hmiddle !== 0) {
    // find the widest point (top, bottom, or theta=0)
    let indexInsert = 0;
    while (ellipsoid[0][indexInsert].z < 0) {
      indexInsert++;
    }
    console.log(indexInsert);
    divisions = divisions + 1;
    for (let indexp = 0; indexp <= Divisions; indexp++) {
      ellipsoid[indexp].splice(
        indexInsert,
        0,
        JSON.parse(JSON.stringify(ellipsoid[indexp][indexInsert]))
      ); // double the theta = 0 point
      for (let indext = 0; indext <= indexInsert; indext++) {
        // shift bottom half of ellipsoid down
        ellipsoid[indexp][indext].z -= hmiddle / 2;
      }
      for (let indext = indexInsert + 1; indext <= divisions; indext++) {
        // shift top half of ellipsoid up
        ellipsoid[indexp][indext].z += hmiddle / 2;
      }
    }
  }

  if (htop !== 0) {
    let indexInsert = divisions;
    for (let indexp = 0; indexp <= Divisions; indexp++) {
      ellipsoid[indexp].push({
        x: ellipsoid[indexp][indexInsert].x * htopfraction + htopshift,
        y: ellipsoid[indexp][indexInsert].y * htopfraction,
        z: ellipsoid[indexp][indexInsert].z + htop
      });
    }
    divisions = divisions + 1;
  }

  if (hbottom !== 0) {
    let indexInsert = 0;
    for (let indexp = 0; indexp <= Divisions; indexp++) {
      // insert point to add height value
      ellipsoid[indexp].unshift({
        x: ellipsoid[indexp][indexInsert].x,
        y: ellipsoid[indexp][indexInsert].y,
        z: ellipsoid[indexp][indexInsert].z - hbottom
      });
    }
    divisions = divisions + 1;
  }

  // --------------------------------------------------------------------------
  // Find widest point along ellipsoid
  // --------------------------------------------------------------------------
  let indexWide = 0;
  if (theta_min >= 0) {
    indexWide = 0;
  } else if (theta_max < 0) {
    indexWide = divisions;
  } else {
    while (ellipsoid[0][indexWide].z < 0) {
      indexWide++;
    }
    indexWide -= 1;
  }

  console.log("Ellipsoid");
  console.log(ellipsoid);

  // --------------------------------------------------------------------------
  // Create panel object
  // --------------------------------------------------------------------------

  // step through all thetas starting as the top of the ellipsoid (theta closest to pi/2)
  // at each height (theta) loop through all
  const panels = [];
  for (let indexp = 0; indexp < Divisions; indexp++) {
    // length of phi is 1 larger than number of panels
    panels[indexp] = [];
    for (let indext = 0; indext <= divisions; indext++) {
      panels[indexp][indext] = [
        ellipsoid[indexp][indext],
        ellipsoid[indexp + 1][indext]
      ];
    }
  }

  console.log("Panels");
  console.log(panels);

  // Populate 3D plot
  //drawVisualization(panels, "mygraph");

  const [panelsMin, panelsMax] = panelExtents(panels);

  console.log("Divisions " + Divisions + "  divisions " + divisions);
  console.log("Max z : " + panelsMax.z + " Min z : " + panelsMin.z);

  // step through all the panels and flatten them
  // Flatten panel strips one at a time.  For each panel strip, loop through all the angles (theta)
  // starting at the bottom of the half-ellipsoid (theta = 0) and moving up toward theta=pi/2.
  // At each theta calculate the angle between the current panel and the one above it (wrt z axis).
  // Rotate each point in the panel including and below the current panel by the found angle between planes.

  // --------------------------------------------------------------------------
  // Flatten the panels
  // --------------------------------------------------------------------------

  //const panelsFlat = jQuery.extend(true, [], panels);
  // const panelsFlat = Array.from(panels);
  //const panelsFlat = Object.assign({},panels);
  const panelsFlat = cloneDeep(panels);

  switch (projection) {

    case "spherical":

      for (let indexp = 0; indexp < Divisions; indexp++) {
        for (let indext = 0; indext < divisions; indext++) {
          // rotation happens around the indext+1 line
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const topPoint = (indext === divisions - 1) ? {
            x: 0,
            y: 0,
            z: panelsMax.z
          } : panelsFlat[indexp][indext + 2][1];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(panelsFlat[indexp][indext + 1][0], panelsFlat[indexp][indext + 1][1], panelsFlat[indexp][indext][0], topPoint);
          // console.log("Index p: "+indexp+" t: "+indext);
          // console.log(panels[indexp][indext + 1][0]);
          // console.log(panels[indexp][indext + 1][1]);
          // console.log(panels[indexp][indext][0]);
          // console.log(topPoint);
          // console.log("angle " + rotationAngle*180/pi);

          if (htop > 0 && indext === divisions - 2 && theta_max > 0) {
            rotationAngle = -rotationAngle;
          }
          if (htop > 0 && indext === divisions - 1) {
            // if htop is used then the last angle of rotation is 90deg
            rotationAngle = Math.PI / 2;
          }
          // if this is a case where all the thetas are negative and its the "top" of the panel then rotate by
          // the obtuse angle between the planes not the acute angle as determined by angleBetweenPlanes
          if (hbottom > 0 && indext === 0 && theta_min < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indextR = 0; indextR <= indext; indextR++) {
            panelsFlat[indexp][indextR][0] = rotatePoint(panelsFlat[indexp][indext + 1][1], panelsFlat[indexp][indext + 1][0], panelsFlat[indexp][indextR][0], rotationAngle);
            panelsFlat[indexp][indextR][1] = rotatePoint(panelsFlat[indexp][indext + 1][1], panelsFlat[indexp][indext + 1][0], panelsFlat[indexp][indextR][1], rotationAngle);
          }
        }
      }
      break;
    case "cylindrical":
      // 1 - figure out the widest area of the ellipsoid
      // hInsert // name of height location (bottom, middle, top)
      // indexh // index location of added height
      // indexWide // index location of widest point on surface

      // 2 - unwrap each panel around the widest part of the cylinder
      for (let indexp = 0; indexp < Divisions; indexp++) {
        for (let indext = 0; indext < indexWide; indext++) {
          // rotation happens around the indext+1 line
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const topPoint = (indext === divisions - 1) ? {
            x: 0,
            y: 0,
            z: panelsMax.z
          } : panels[indexp][indext + 2][1];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(panels[indexp][indext + 1][0], panels[indexp][indext + 1][1], panels[indexp][indext][0], topPoint);
          //console.log(panels[indexp][indext + 1][0]);
          //console.log(panels[indexp][indext + 1][1]);
          //console.log(panels[indexp][indext][0]);
          //console.log(topPoint);
          //console.log(rotationAngle * 180 / pi);

          if (htop > 0 && indext === divisions && theta_max > 0) {
            rotationAngle = -rotationAngle;
          }

          // if this is a case where all the thetas are negative and its the "top" of the panel then rotate by
          // the obtuse angle between the planes not the acute angle as determined by angleBetweenPlanes
          if (hbottom > 0 && indext === 0 && theta_min < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indextR = 0; indextR <= indext; indextR++) {
            panelsFlat[indexp][indextR][0] = rotatePoint(panels[indexp][indext + 1][1], panels[indexp][indext + 1][0], panelsFlat[indexp][indextR][0], rotationAngle);
            panelsFlat[indexp][indextR][1] = rotatePoint(panels[indexp][indext + 1][1], panels[indexp][indext + 1][0], panelsFlat[indexp][indextR][1], rotationAngle);
          }
        }

        for (let indext = divisions; indext > indexWide + 1; indext--) {
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const bottomPoint = panels[indexp][indext - 2][0];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(panels[indexp][indext - 1][0], panels[indexp][indext - 1][1], bottomPoint, panels[indexp][indext][0]);

          if (htop > 0 && indext === divisions && theta_max > 0) {
            rotationAngle = -rotationAngle;
          }
          if (hbottom > 0 && indext === 0 && theta_min < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indextR = divisions; indextR >= indext; indextR--) {
            panelsFlat[indexp][indextR][0] = rotatePoint(panels[indexp][indext - 1][0], panels[indexp][indext - 1][1], panelsFlat[indexp][indextR][0], rotationAngle);
            panelsFlat[indexp][indextR][1] = rotatePoint(panels[indexp][indext - 1][0], panels[indexp][indext - 1][1], panelsFlat[indexp][indextR][1], rotationAngle);
          }
        }
      }

      // 3 - unwrap the cylinder
      for (let indexp = 0; indexp < Divisions; indexp++) {
        const prevPanel = (indexp === 0) ? (Divisions - 1) : (indexp - 1);

        // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
        let rotationAngle = angleBetweenPlanes(panelsFlat[indexp][indexWide][0], panelsFlat[indexp][indexWide + 1][0], panelsFlat[indexp][indexWide][1], panelsFlat[prevPanel][indexWide][0]);
        if (indexp === 0) {
          // since phi always starts at 0 we know that we need to rotate half the angle betwee the planes on the first panel.
          // this moves the flattened pattern points to a plane along the min x value (where phi=0)
          rotationAngle = rotationAngle / 2;
        }

        for (let indexpR = indexp; indexpR < Divisions; indexpR++) {
          for (let indext = 0; indext <= divisions; indext++) {
            panelsFlat[indexpR][indext][0] = rotatePoint(panelsFlat[indexp][indexWide + 1][0], panelsFlat[indexp][indexWide][0], panelsFlat[indexpR][indext][0], rotationAngle);
            panelsFlat[indexpR][indext][1] = rotatePoint(panelsFlat[indexp][indexWide + 1][0], panelsFlat[indexp][indexWide][0], panelsFlat[indexpR][indext][1], rotationAngle);
          }
        }
      }
      break;

    default:
      console.log("ERROR - Projection Type");
  }

  console.log("Panels Flattened");
  console.log(panelsFlat);

  return {
    panels: panels,
    panelsFlat: panelsFlat,
    indexWide: indexWide
  };
}


export function drawPattern(state, ellipsoid, scope) {

  const projection = state.projection;
  const ppu = parseFloat(state.ppu);
  const htop = parseFloat(state.hTop);
  const mingap = parseFloat(state.minGap);
  const Divisions = parseFloat(state.Divisions);
  const divisions = parseFloat(state.divisions);
  const image_offset = parseFloat(state.imageOffset);

  const panelsFlat = cloneDeep(ellipsoid.panelsFlat);

  const indexWide = ellipsoid.indexWide;

  // calculate a bounding box around the flattened pattern

  const [panelsFlatMin, panelsFlatMax] = panelExtents(panelsFlat);

  console.log(
    "Flat Pattern Bounding Box " +
    pointToString(panelsFlatMin) +
    " " +
    pointToString(panelsFlatMax)
  );

  // Populate 3D plot
  //drawVisualization(panelsFlat, "mygraphF");

  const image = {
    width: 0,
    height: 0
  };

  // computer the center point of the image
  const shift = {
    x: 0,
    y: 0
  };

  switch (projection) {
    case "spherical":
      image.width = (panelsFlatMax.x - panelsFlatMin.x + 2 * image_offset) * ppu;
      image.height = (panelsFlatMax.y - panelsFlatMin.y + 2 * image_offset) * ppu;
      shift.x = (panelsFlatMax.x - panelsFlatMin.x) / 2 + image_offset;
      shift.y = (panelsFlatMax.y - panelsFlatMin.y) / 2 + image_offset;
      break;
    case "cylindrical":
      image.width = (panelsFlatMax.y - panelsFlatMin.y + 2 * image_offset) * ppu;
      image.height = (panelsFlatMax.z - panelsFlatMin.z + 2 * image_offset) * ppu;
      shift.x = Math.abs(panelsFlatMin.y) + image_offset;
      shift.y = image_offset;
      break;
    default:
      console.log("ERROR - Projection Type");
  }



  const strokeWidth = 3 * ppu / 90; // enforce that the stroke width be scaled with the units of the drawing

  const patternLayer = scope.project.activeLayer;
  const backgroundLayer = new scope.Layer();
  backgroundLayer.name = 'Bounding Box';
  backgroundLayer.activate();

  var boundingRect = new scope.Shape.Rectangle(new scope.Point(0, 0), new scope.Point(image.width, image.height));
  boundingRect.strokeColor = "#333333";
  boundingRect.fillColor = new scope.Color(1, 0, 0.5, 0);

  patternLayer.activate();

  // -----------------------------------------------------------------------------
  // assemble ordered points array for the flat pattern
  // -----------------------------------------------------------------------------
  // Loop through all the panels.  For each panel draw the ending edge of the previous panel then
  // the starting edge of the current panel.  This ensures the cutouts are drawn as the outlines as
  // opposed to drawing the panels themselves.
  const points_full = [];

  switch (projection) {
    case "spherical":
      for (let indexp = 0; indexp < Divisions; indexp++) {
        // Figure out the index for the previous and the next panels.  This is used to enforce mingap
        const idxPhiPrev = (indexp - 1 < 0) ? Divisions - 1 : indexp - 1;

        for (let indext = 0; indext <= divisions; indext++) {
          if (htop > 0 && indext === divisions) {
            // the mingap check fails if there is added height to the ellipsoid on top so a special case is needed
            points_full.push([shift.x + panelsFlat[idxPhiPrev][indext][1].x, shift.y + panelsFlat[idxPhiPrev][indext][1].y]);
          } else if (distance(panelsFlat[idxPhiPrev][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[idxPhiPrev][indext][1].x, shift.y + panelsFlat[idxPhiPrev][indext][1].y]);
          }
        }
        for (let indext = divisions; indext >= 0; indext--) {
          if (htop > 0 && indext === divisions) {
            // the mingap check fails if there is added height to the ellipsoid on top so a special case is needed
            points_full.push([shift.x + panelsFlat[indexp][indext][0].x, shift.y + panelsFlat[indexp][indext][0].y]);
          } else if (distance(panelsFlat[idxPhiPrev][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][0].x, shift.y + panelsFlat[indexp][indext][0].y]);
          }
        }
      }
      break;
    case "cylindrical":
      console.log("wide " + indexWide);
      // let count = 0;

      for (let indexp = 0; indexp < Divisions; indexp++) {
        const idxPhiPrev = indexp === 0 ? Divisions - 1 : indexp - 1;
        const idxPhiNext = indexp + 1 === Divisions ? 0 : indexp + 1;

        for (let indext = indexWide; indext >= 0; indext--) {
          if (distance(panelsFlat[idxPhiPrev][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][0].y, shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][0].z]);
            // s.text((shift.x + panelsFlat[indexp][indext][0].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][0].z)*ppu,"b "+indexp+" "+indext+" "+count).attr({'fill' : 'green',  'stroke': 'green'});
            // count += 1;
          }
        }
        for (let indext = 0; indext <= indexWide; indext++) {
          if (distance(panelsFlat[idxPhiNext][indext][0], panelsFlat[indexp][indext][1]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][1].y, shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][1].z]);
            // s.text((shift.x + panelsFlat[indexp][indext][1].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][1].z)*ppu,"d "+indexp+" "+indext+" "+count).attr({'fill' : 'yellow',  'stroke': 'yellow'});
            // count += 1;
          }
        }
      }
      for (let indexp = Divisions - 1; indexp >= 0; indexp--) {
        const idxPhiPrev = indexp + 1 === Divisions ? 0 : indexp + 1;
        const idxPhiNext = indexp === 0 ? Divisions - 1 : indexp - 1;

        for (let indext = indexWide; indext <= divisions; indext++) {
          if (distance(panelsFlat[idxPhiPrev][indext][0], panelsFlat[indexp][indext][1]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][1].y, shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][1].z]);
            // s.text((shift.x + panelsFlat[indexp][indext][1].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][1].z)*ppu,"f "+indexp+" "+indext+" "+count).attr({'fill' : 'pink',  'stroke': 'pink'});
            // count += 1;
          }
        }
        for (let indext = divisions; indext > indexWide; indext--) {
          if (distance(panelsFlat[idxPhiNext][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][0].y, shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][0].z]);
            // s.text((shift.x + panelsFlat[indexp][indext][0].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][0].z)*ppu,"h "+indexp+" "+indext+" "+count).attr({'fill' : 'orange',  'stroke': 'orange'});
            // count += 1;
          }
        }
      }

      break;
    default:
      console.log("ERROR - Projection Type");
  }


  // -----------------------------------------------------------------------------
  // draw the flat pattern (assemble path point string for cutout)
  // -----------------------------------------------------------------------------
  var pointString = "";

  points_full.forEach(function(point, idx) {
    let command = "";
    if (idx === 0) {
      command = "M";
    } else {
      command = "L";
    }
    pointString +=
      command +
      (point[0] * ppu).toFixed(3).toString() + "," +
      (point[1] * ppu).toFixed(3).toString() + " ";
  });
  pointString += " z";
  // add path to drawing

  patternLayer.activate(); // ensure the pattern layer is active
  var path = new scope.Path(pointString);
  path.strokeColor = new scope.Color(0, 0, 0);
  path.strokeWidth = strokeWidth * 0.5;
  path.fillColor = new scope.Color(1, 1, 1, .3);


  // -----------------------------------------------------------------------------
  // draw glueing guide lines on the pattern
  // -----------------------------------------------------------------------------
  const guideLineLayer = new scope.Layer();
  guideLineLayer.name = 'Guide Lines';
  guideLineLayer.activate();

  // add guide lines for gluing
  for (let indexp = 0; indexp < Divisions; indexp++) {

    for (let indext = 1; indext <= divisions; indext++) {
      var line = new scope.Path({
        strokeColor: new scope.Color(0, 1, 0),
        strokeWidth: strokeWidth * 0.25
      })
      switch (projection) {
        case "spherical":
          line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y + panelsFlat[indexp][indext][0].y) * ppu));
          line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y + panelsFlat[indexp][indext][1].y) * ppu));
          break;
        case "cylindrical":
          line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][0].y) * ppu, (shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][0].z) * ppu));
          line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][1].y) * ppu, (shift.y + panelsFlatMax.z - panelsFlat[indexp][indext][1].z) * ppu));
          break;
        default:
          console.log("ERROR - Projection Type");
      }
    }
  }
  patternLayer.activate();
}