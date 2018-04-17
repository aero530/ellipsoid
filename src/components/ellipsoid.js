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


// Get the average (midpoint) between two paper.js points
function averagePoints(point1, point2) {
  var result = point1.add(point2).divide(2);
  return result;
}

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

export function computeGeometry(geometrySettings) {
  // Reference equations for an ellipsoid
  // x = a cos(theta) cos(phi)
  // y = b cos(theta) sin(phi)
  // z = c sin(theta)
  // -pi/2 <= theta <= pi/2 (use 0 to pi/2 to get the top half)
  // -pi <= phi <= pi
  const a = geometrySettings.a;
  const b = geometrySettings.b;
  const c = geometrySettings.c;
  // const theta_min = geometrySettings.thetaMin * Math.PI / 180;
  const theta_min = (geometrySettings.thetaMin === -90) ? -89 * Math.PI / 180 : geometrySettings.thetaMin * Math.PI / 180;
  // const theta_max = geometrySettings.thetaMax * Math.PI / 180;
  const theta_max = (geometrySettings.thetaMax === 90) ? 89 * Math.PI / 180 : geometrySettings.thetaMax * Math.PI / 180;
  

  // const htop = geometrySettings.hTop;
  const htop = (theta_max <= 0 && geometrySettings.hTop === 0) ? .001 : geometrySettings.hTop;
  // hMiddle can't be zero if cylindrical projection is used so make it an insubstantially small number instead
  const hmiddle = geometrySettings.hMiddle === 0 ? .001 : geometrySettings.hMiddle; 
  const hbottom = geometrySettings.hBottom;
  const htopfraction = geometrySettings.hTopFraction;
  const htopshift = geometrySettings.hTopShift;
  let Divisions = geometrySettings.Divisions;
  let divisions = geometrySettings.divisions;

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

  console.debug("thetas");
  console.debug(thetas);
  console.debug("phis");
  console.debug(phis);

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

  if (hmiddle !== 0 && (theta_max>0 && theta_min<0) ) { // if there is hmiddle specified and range of theta spans 0
    // find the widest point (top, bottom, or theta=0)
    let indexInsert = 0;
    while (ellipsoid[0][indexInsert].z < 0) {
      indexInsert++;
    }
    console.debug(indexInsert);
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
  } else if (theta_max <= 0) {
    indexWide = divisions-1; // widest point is at top
  } else {
    while (ellipsoid[0][indexWide].z < 0) {
      indexWide++;
    }
    indexWide -= 1;
  }

  console.debug("Ellipsoid");
  console.debug(ellipsoid);

  return {
    geometry: ellipsoid,
    indexWide: indexWide,
    divisions: divisions,
    Divisions: Divisions
  };
}

export function computePattern(geometry, geometrySettings, projection) {

  // const theta_min = geometrySettings.thetaMin * Math.PI / 180;
  const theta_min = (geometrySettings.thetaMin === -90) ? -89 * Math.PI / 180 : geometrySettings.thetaMin * Math.PI / 180;
  // const theta_max = geometrySettings.thetaMax * Math.PI / 180;
  const theta_max = (geometrySettings.thetaMax === 90) ? 89 * Math.PI / 180 : geometrySettings.thetaMin * Math.PI / 180;
  
  const htop = (theta_max <= 0 && geometrySettings.hTop === 0) ? 0.0001 : geometrySettings.hTop;
  const hbottom = geometrySettings.hBottom;

  const Divisions = geometry.Divisions;
  const divisions = geometry.divisions;
  const ellipsoid = geometry.geometry;
  const indexWide = geometry.indexWide;

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

  console.debug("Panels");
  console.debug(panels);

  const [panelsMin, panelsMax] = panelExtents(panels);

  console.debug("Divisions " + Divisions + "  divisions " + divisions);
  console.debug("Max z : " + panelsMax.z + " Min z : " + panelsMin.z);

  // step through all the panels and flatten them
  // Flatten panel strips one at a time.  For each panel strip, loop through all the angles (theta)
  // starting at the bottom of the half-ellipsoid (theta = 0) and moving up toward theta=pi/2.
  // At each theta calculate the angle between the current panel and the one above it (wrt z axis).
  // Rotate each point in the panel including and below the current panel by the found angle between planes.

  // --------------------------------------------------------------------------
  // Flatten the panels
  // --------------------------------------------------------------------------
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
      console.error("ERROR - Projection Type");
  }

  console.debug("Panels Flattened");
  console.debug(panelsFlat);


  switch (projection) {
    case "spherical":
      return {panelsFlat: panelsFlat, panels: panels, indexWide: indexWide};
    case "cylindrical":
      // reorder the points of the flattened panels to be in the x/y domain
      let output = Array.from(panelsFlat,function(val1,idx1) {
        return (
          Array.from(val1, function(val2,idx2){
            return Array.from(val2, function(val3,idx3){
              return {x:val3.y*-1, y:val3.z, z:val3.x };
            })
          })
        );
      });
      return {panelsFlat: output, panels: panels, indexWide: indexWide};
    default:
      console.error("ERROR - Projection Type");
  }
}

export function drawPattern(geometrySettings, projectionSettings, pattern, scope) {

  const projection = projectionSettings.projection;
  const ppu = geometrySettings.ppu;
  const htop = geometrySettings.hTop;
  const mingap = projectionSettings.minGap;
  const image_offset = projectionSettings.imageOffset;

  const panelsFlat = cloneDeep(pattern.panelsFlat);

  const indexWide = pattern.indexWide;
  const Divisions = pattern.panelsFlat.length;
  const divisions = pattern.panelsFlat[0].length-1;

  // calculate a bounding box around the flattened pattern

  const [panelsFlatMin, panelsFlatMax] = panelExtents(panelsFlat);

  console.debug(
    "Flat Pattern Bounding Box " +
    pointToString(panelsFlatMin) +
    " " +
    pointToString(panelsFlatMax)
  );

  const image = {
    width: 0,
    height: 0
  };

  // computer the center point of the image
  const shift = {
    x: 0,
    y: 0
  };

  image.width = (panelsFlatMax.x - panelsFlatMin.x + 2 * image_offset) * ppu;
  image.height = (panelsFlatMax.y - panelsFlatMin.y + 2 * image_offset) * ppu;

  switch (projection) {
    case "spherical":
      shift.x = (panelsFlatMax.x - panelsFlatMin.x) / 2 + image_offset;
      shift.y = (panelsFlatMax.y - panelsFlatMin.y) / 2 + image_offset;
      break;
    case "cylindrical":
      shift.x = Math.abs(panelsFlatMin.x) + image_offset;
      shift.y = image_offset + panelsFlatMax.y;
      break;
    default:
      console.error("ERROR - Projection Type");
  }

  const strokeWidth = 3 * ppu / 90; // enforce that the stroke width be scaled with the units of the drawing

  const patternLayer = scope.project.activeLayer;
  const backgroundLayer = new scope.Layer();
  backgroundLayer.name = 'Bounding Box';
  backgroundLayer.activate();

  var boundingRect = new scope.Shape.Rectangle(new scope.Point(0, 0), new scope.Point(image.width, image.height));
  boundingRect.strokeColor = "#333333";
  boundingRect.strokeWidth = 0.01*ppu;
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
      console.debug("wide " + indexWide);
      // let count = 0;

      for (let indexp = 0; indexp < Divisions; indexp++) {
        const idxPhiPrev = indexp === 0 ? Divisions - 1 : indexp - 1;
        const idxPhiNext = indexp + 1 === Divisions ? 0 : indexp + 1;

        for (let indext = indexWide; indext >= 0; indext--) {
          if (distance(panelsFlat[idxPhiPrev][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][0].x, shift.y - panelsFlat[indexp][indext][0].y]);
            // s.text((shift.x + panelsFlat[indexp][indext][0].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][0].z)*ppu,"b "+indexp+" "+indext+" "+count).attr({'fill' : 'green',  'stroke': 'green'});
            // count += 1;
          }
        }
        for (let indext = 0; indext <= indexWide; indext++) {
          if (distance(panelsFlat[idxPhiNext][indext][0], panelsFlat[indexp][indext][1]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][1].x, shift.y - panelsFlat[indexp][indext][1].y]);
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
            points_full.push([shift.x + panelsFlat[indexp][indext][1].x, shift.y - panelsFlat[indexp][indext][1].y]);
            // s.text((shift.x + panelsFlat[indexp][indext][1].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][1].z)*ppu,"f "+indexp+" "+indext+" "+count).attr({'fill' : 'pink',  'stroke': 'pink'});
            // count += 1;
          }
        }
        for (let indext = divisions; indext > indexWide; indext--) {
          if (distance(panelsFlat[idxPhiNext][indext][1], panelsFlat[indexp][indext][0]) > mingap) {
            // Enforce minimum gap
            points_full.push([shift.x + panelsFlat[indexp][indext][0].x, shift.y - panelsFlat[indexp][indext][0].y]);
            // s.text((shift.x + panelsFlat[indexp][indext][0].y)*ppu, (shift.y + panelsFlatMax.z-panelsFlat[indexp][indext][0].z)*ppu,"h "+indexp+" "+indext+" "+count).attr({'fill' : 'orange',  'stroke': 'orange'});
            // count += 1;
          }
        }
      }

      break;
    default:
      console.error("ERROR - Projection Type");
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
  var group = new scope.Group();
  for (let indexp = 0; indexp < Divisions; indexp++) {
    for (let indext = 1; indext <= divisions; indext++) {
      let line = new scope.Path({
        strokeColor: new scope.Color(0, 1, 0),
        strokeWidth: strokeWidth * 0.25
      })
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu));
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu));
      group.addChild(line);
    }
  }

  // -----------------------------------------------------------------------------
  // draw pattern clip quadrilaterals
  // -----------------------------------------------------------------------------
  const quadLayer = new scope.Layer();
  quadLayer.name = 'Pattern Destination Quadrilaterals';
  quadLayer.activate();
  for (let indexp = 0; indexp < Divisions; indexp++) {
    for (let indext = 0; indext < divisions; indext++) {
      let line = new scope.Path({
        strokeColor: new scope.Color(1, 0, 0),
        strokeWidth: strokeWidth * 0.25
      })
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu));
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu));
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu));
      line.add(new scope.Point((shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu));
      line.closed = true;
    }
  }
  
  // -----------------------------------------------------------------------------
  // draw source pattern quadrilaterals
  // -----------------------------------------------------------------------------
  const quadSourceLayer = new scope.Layer();
  quadSourceLayer.name = 'Pattern Source Quadrilaterals';
  quadSourceLayer.activate();

  const leftX = panelsFlat[0][indexWide][0].x;
  const rightX = panelsFlat[Divisions-1][indexWide][1].x;

  for (let indexp = 0; indexp < Divisions; indexp++) {
    for (let indext = 0; indext < divisions; indext++) {
      let line = new scope.Path({
        strokeColor: new scope.Color(1, 0, 1),
        strokeWidth: strokeWidth * 0.25
      })

      const P1a = new scope.Point();
      const P1b = new scope.Point();
      const P2a = new scope.Point();
      const P2b = new scope.Point();
      const P3a = new scope.Point();
      const P3b = new scope.Point();
      const P4a = new scope.Point();
      const P4b = new scope.Point();

      // Since the first and last panels (Divisions) connect to eachother and are always symmetrical, the pattern mapping along
      // the left and right edges will be a vertical line.  The if block below enforces that symmetry.
      if (indexp === 0) {
        if (projection ==="spherical") {
          P1a.set( (shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu);
          P1b.set( (shift.x + panelsFlat[Divisions-1][indext][1].x) * ppu, (shift.y - panelsFlat[Divisions-1][indext][1].y) * ppu);
          P2a.set( (shift.x + panelsFlat[indexp][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu);
          P2b.set( (shift.x + panelsFlat[Divisions-1][indext+1][1].x) * ppu, (shift.y - panelsFlat[Divisions-1][indext+1][1].y) * ppu);
        } else {
          P1a.set( (shift.x + leftX) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu);
          P1b.set( (shift.x + leftX) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu);
          P2a.set( (shift.x + leftX) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu);
          P2b.set( (shift.x + leftX) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu);
        }
        P3a.set( (shift.x + panelsFlat[indexp][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu);
        P3b.set( (shift.x + panelsFlat[indexp+1][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp+1][indext+1][0].y) * ppu);
        P4a.set( (shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu);
        P4b.set( (shift.x + panelsFlat[indexp+1][indext][0].x) * ppu, (shift.y - panelsFlat[indexp+1][indext][0].y) * ppu);
      } else if (indexp === Divisions-1) {
        P1a.set( (shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu);
        P1b.set( (shift.x + panelsFlat[indexp-1][indext][1].x) * ppu, (shift.y - panelsFlat[indexp-1][indext][1].y) * ppu);
        P2a.set( (shift.x + panelsFlat[indexp][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu);
        P2b.set( (shift.x + panelsFlat[indexp-1][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp-1][indext+1][1].y) * ppu);
        if (projection === "spherical") {
          P3a.set( (shift.x + panelsFlat[indexp][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu);
          P3b.set( (shift.x + panelsFlat[0][indext+1][0].x) * ppu, (shift.y - panelsFlat[0][indext+1][0].y) * ppu);
          P4a.set( (shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu);
          P4b.set( (shift.x + panelsFlat[0][indext][0].x) * ppu, (shift.y - panelsFlat[0][indext][0].y) * ppu);
        } else {
          P3a.set( (shift.x + rightX) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu);
          P3b.set( (shift.x + rightX) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu);
          P4a.set( (shift.x + rightX) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu);
          P4b.set( (shift.x + rightX) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu);
        }
        
      } else {
        P1a.set( (shift.x + panelsFlat[indexp][indext][0].x) * ppu, (shift.y - panelsFlat[indexp][indext][0].y) * ppu);
        P1b.set( (shift.x + panelsFlat[indexp-1][indext][1].x) * ppu, (shift.y - panelsFlat[indexp-1][indext][1].y) * ppu);
        P2a.set( (shift.x + panelsFlat[indexp][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][0].y) * ppu);
        P2b.set( (shift.x + panelsFlat[indexp-1][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp-1][indext+1][1].y) * ppu);
        P3a.set( (shift.x + panelsFlat[indexp][indext+1][1].x) * ppu, (shift.y - panelsFlat[indexp][indext+1][1].y) * ppu);
        P3b.set( (shift.x + panelsFlat[indexp+1][indext+1][0].x) * ppu, (shift.y - panelsFlat[indexp+1][indext+1][0].y) * ppu);
        P4a.set( (shift.x + panelsFlat[indexp][indext][1].x) * ppu, (shift.y - panelsFlat[indexp][indext][1].y) * ppu);
        P4b.set( (shift.x + panelsFlat[indexp+1][indext][0].x) * ppu, (shift.y - panelsFlat[indexp+1][indext][0].y) * ppu);
      }

      line.add(averagePoints(P1a,P1b));
      line.add(averagePoints(P2a,P2b));
      line.add(averagePoints(P3a,P3b));
      line.add(averagePoints(P4a,P4b));
      line.closed = true;
    }
  }
  patternLayer.activate();
}

export function drawNotes(scope, props) {
  var notesLayer = new scope.Layer();
  notesLayer.name = 'Notes';
  notesLayer.activate();

  let units = getUnits(props.ppu);

  const filename = "ellipsoid_a" + props.a + units +
      "_b" + props.b + units +
      "_c" + props.c + units +
      ".svg";

  let textFilename = new scope.PointText({
          point: [0, 0],
          content: filename,
          fillColor: 'black',
          fontFamily: 'Roboto',
          fontSize: 0.25*props.ppu
      });
      
      textFilename.rotate(-90, textFilename.bounds.bottomRight);
      textFilename.position.x = 0.15*props.ppu;
      textFilename.position.y = scope.project.layers['Bounding Box'].bounds.height - textFilename.bounds.height/2 - 0.15*props.ppu;

  new scope.PointText({
      point: [0.1*props.ppu, .15*props.ppu],
      content: JSON.stringify(props.geometrySettings),
      fillColor: 'black',
      fontFamily: 'Courier New',
      fontSize: 0.2*props.ppu
  });

  // Draw a ruler on the bottom of the pattern based on the units specified

  var path = new scope.Path();
  // Give the stroke a color
  path.strokeColor = new scope.Color(.7,.3,.5);
  path.strokeWidth = 0.01*props.ppu;
  // var start = new scope.Point(0.1*props.ppu, scope.project.layers['Bounding Box'].bounds.height);
  var start = new scope.Point(scope.project.layers['Ellipsoid Pattern'].bounds.x, scope.project.layers['Bounding Box'].bounds.height);
  // Move to start and draw a line from there
  path.moveTo(start);
  // Note that the plus operator on Point objects does not work
  // in JavaScript. Instead, we need to call the add() function:
  path.lineTo(start.add([ 0, -0.3*props.ppu ]));

  for (var i = 0; i < scope.project.layers['Ellipsoid Pattern'].bounds.width / props.ppu; i++) {
      var copy = path.clone();
      // Distribute the copies horizontally, so we can see them:
      copy.position.x += i * props.geometrySettings.ppu;
      new scope.PointText({
          point: copy.position,
          content: i+units,
          fillColor: 'black',
          fontFamily: 'Roboto',
          fontSize: 0.2*props.ppu
      });
  }
}

export function getUnits(ppu) {
  if (ppu === 96 || ppu === "96") {
      return "in";
  } else if (ppu === 3.7795276 || ppu === "3.7795276") {
      return "mm";
  } else if (ppu === 37.795276 || ppu === "37.795276") {
      return "cm";
  }
}