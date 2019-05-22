// cSpell: ignore indexP, indexT

import cloneDeep from 'lodash.clonedeep';
import {
  angleBetweenPlanes,
  pointToString,
  rotatePoint,
  distance,
} from './geometryHelpers';
import { settings } from 'paper';

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
  return point1.add(point2).divide(2);
}

function edgeGeometryExtents(array) {
  const arrayMax = {
    x: Number.NEGATIVE_INFINITY,
    y: Number.NEGATIVE_INFINITY,
    z: Number.NEGATIVE_INFINITY,
  };
  const arrayMin = {
    x: Number.POSITIVE_INFINITY,
    y: Number.POSITIVE_INFINITY,
    z: Number.POSITIVE_INFINITY,
  };

  for (let indexP = 0; indexP < array.length; indexP += 1) {
    for (let indexT = 0; indexT < array[indexP].length; indexT += 1) {
      if (array[indexP][indexT][0].x > arrayMax.x) {
        arrayMax.x = array[indexP][indexT][0].x;
      }
      if (array[indexP][indexT][1].x > arrayMax.x) {
        arrayMax.x = array[indexP][indexT][1].x;
      }

      if (array[indexP][indexT][0].y > arrayMax.y) {
        arrayMax.y = array[indexP][indexT][0].y;
      }
      if (array[indexP][indexT][1].y > arrayMax.y) {
        arrayMax.y = array[indexP][indexT][1].y;
      }

      if (array[indexP][indexT][0].z > arrayMax.z) {
        arrayMax.z = array[indexP][indexT][0].z;
      }
      if (array[indexP][indexT][1].z > arrayMax.z) {
        arrayMax.z = array[indexP][indexT][1].z;
      }

      if (array[indexP][indexT][0].x < arrayMin.x) {
        arrayMin.x = array[indexP][indexT][0].x;
      }
      if (array[indexP][indexT][1].x < arrayMin.x) {
        arrayMin.x = array[indexP][indexT][1].x;
      }

      if (array[indexP][indexT][0].y < arrayMin.y) {
        arrayMin.y = array[indexP][indexT][0].y;
      }
      if (array[indexP][indexT][1].y < arrayMin.y) {
        arrayMin.y = array[indexP][indexT][1].y;
      }

      if (array[indexP][indexT][0].z < arrayMin.z) {
        arrayMin.z = array[indexP][indexT][0].z;
      }
      if (array[indexP][indexT][1].z < arrayMin.z) {
        arrayMin.z = array[indexP][indexT][1].z;
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
  let { a, b, c, thetaMin, thetaMax, hTop, hMiddle, hBottom, hTopFraction, hTopShift, Divisions, divisions } = geometrySettings;

  a = isNaN(a) ? 0 : a; // check to make sure a is a number
  b = isNaN(b) ? 0 : b;
  c = isNaN(c) ? 0 : c;

  thetaMin = isNaN(thetaMin) ? -90 : thetaMin;
  thetaMin = (thetaMin === -90) ? -89 * Math.PI / 180 : thetaMin * Math.PI / 180;
  
  thetaMax = isNaN(thetaMax) ? 90 : thetaMax;
  thetaMax = (thetaMax === 90) ? 89 * Math.PI / 180 : thetaMax * Math.PI / 180;

  hTop = isNaN(hTop) ? 0 : hTop;
  hTop = (thetaMax <= 0 && hTop === 0) ? 0.001 : hTop;

  // hMiddle can't be zero if cylindrical projection is used so make it an insubstantially small number instead
  hMiddle = isNaN(hMiddle) ? 0 : hMiddle;
  hMiddle = hMiddle === 0 ? 0.001 : hMiddle;
  
  hBottom = isNaN(hBottom) ? 0 : hBottom;
  hTopFraction = isNaN(hTopFraction) ? 0 : hTopFraction;
  hTopShift = isNaN(hTopShift) ? 0 : hTopShift;
  Divisions = isNaN(Divisions) ? 4 : Divisions;
  divisions = isNaN(divisions) ? 4 : divisions;

  // -----------------------------------------------------------------------------
  // Compute basic dimensions of the ellipsoid
  // -----------------------------------------------------------------------------

  // -----------------------------------------------------------------------------
  // create array of theta and phi angles
  // -----------------------------------------------------------------------------

  const thetas = Array.apply(null, Array(divisions + 1)).map((_, i) => thetaMin + i * ((thetaMax - thetaMin) / divisions));

  // Enforce that if thetas spans 0 that 0 is included in the array
  if (thetaMax > 0 && thetaMin < 0) {
    let idx = 0;
    while (thetas[idx] < 0) {
      idx += 1;
    }
    if (thetas[idx] !== 0) {
      // if theta=0 is not already in the array
      thetas.splice(idx, 0, 0);
      divisions += 1;
    }
  }

  const phis = Array.apply(null, Array(Divisions + 1)).map((_, i) => -Math.PI + i * (2 * Math.PI / Divisions));

  console.debug('thetas');
  console.debug(thetas);
  console.debug('phis');
  console.debug(phis);

  // --------------------------------------------------------------------------
  // generate all the points that make up the ellipsoid approximation based on the number of divisions specified
  // --------------------------------------------------------------------------
  const ellipsoid = [];
  for (let indexP = 0; indexP <= Divisions; indexP += 1) {
    const phi = phis[indexP];
    ellipsoid[indexP] = [];
    for (let indexT = 0; indexT <= divisions; indexT += 1) {
      const theta = thetas[indexT];
      ellipsoid[indexP][indexT] = {
        x: a * Math.cos(theta) * Math.cos(phi),
        y: b * Math.cos(theta) * Math.sin(phi),
        z: c * Math.sin(theta),
      };
    }
  }

  // --------------------------------------------------------------------------
  // Add height to ellipsoid
  // --------------------------------------------------------------------------

  if (hMiddle !== 0 && (thetaMax > 0 && thetaMin < 0)) { // if there is hMiddle specified and range of theta spans 0
    // find the widest point (top, bottom, or theta=0)
    let indexInsert = 0;
    while (ellipsoid[0][indexInsert].z < 0) {
      indexInsert += 1;
    }
    console.debug(indexInsert);
    divisions += 1;
    for (let indexP = 0; indexP <= Divisions; indexP += 1) {
      ellipsoid[indexP].splice(
        indexInsert,
        0,
        JSON.parse(JSON.stringify(ellipsoid[indexP][indexInsert])),
      ); // double the theta = 0 point
      for (let indexT = 0; indexT <= indexInsert; indexT += 1) {
        // shift bottom half of ellipsoid down
        ellipsoid[indexP][indexT].z -= hMiddle / 2;
      }
      for (let indexT = indexInsert + 1; indexT <= divisions; indexT += 1) {
        // shift top half of ellipsoid up
        ellipsoid[indexP][indexT].z += hMiddle / 2;
      }
    }
  }

  if (hTop !== 0) {
    const indexInsert = divisions;
    for (let indexP = 0; indexP <= Divisions; indexP += 1) {
      ellipsoid[indexP].push({
        x: ellipsoid[indexP][indexInsert].x * hTopFraction + hTopShift,
        y: ellipsoid[indexP][indexInsert].y * hTopFraction,
        z: ellipsoid[indexP][indexInsert].z + hTop,
      });
    }
    divisions += 1;
  }

  if (hBottom !== 0) {
    const indexInsert = 0;
    for (let indexP = 0; indexP <= Divisions; indexP += 1) {
      // insert point to add height value
      ellipsoid[indexP].unshift({
        x: ellipsoid[indexP][indexInsert].x,
        y: ellipsoid[indexP][indexInsert].y,
        z: ellipsoid[indexP][indexInsert].z - hBottom,
      });
    }
    divisions += 1;
  }

  // --------------------------------------------------------------------------
  // Find widest point along ellipsoid
  // --------------------------------------------------------------------------
  let indexWide = 0;
  if (thetaMin >= 0) {
    indexWide = 0;
  } else if (thetaMax <= 0) {
    indexWide = divisions - 1; // widest point is at top
  } else {
    while (ellipsoid[0][indexWide].z < 0) {
      indexWide += 1;
    }
    indexWide -= 1;
  }

  console.debug('Ellipsoid');
  console.debug(ellipsoid);

  // --------------------------------------------------------------------------
  // Generate OBJ ascii
  // --------------------------------------------------------------------------

  let object3D = '';
  // create verticies
  for (let indexP = 0; indexP <= Divisions; indexP += 1) { // the last phi division is a duplicate of the first (for edges)
    for (let indexT = 0; indexT <= divisions; indexT += 1) {
      object3D += `v ${ellipsoid[indexP][indexT].x} ${ellipsoid[indexP][indexT].y} ${ellipsoid[indexP][indexT].z} \n`;
    }
  }
  object3D += `s off \n`;
  // create faces
  
  // for (let indexP = 0; indexP < Divisions; indexP += 1) {
  //   for (let indexT = 0; indexT < divisions; indexT += 1) {
  for (let indexP = 1; indexP <= Divisions; indexP += 1) {
    for (let indexT = 1; indexT <= divisions; indexT += 1) {
      object3D += `# ${indexP} ${indexT} \n`;
      let vertexA = 1;
      let vertexB = 1;
      let vertexC = 1;
      let vertexD = 1;
      vertexA = indexT + (indexP-1)*divisions + (indexP-1);
      vertexB = vertexA+1;
      vertexC = vertexB+(divisions+1);
      vertexD = vertexC-1;
      object3D += `f ${vertexA} ${vertexD} ${vertexC} ${vertexB} \n`;
    }
  }

  return {
    geometry: ellipsoid,
    indexWide,
    divisions,
    Divisions,
    obj: object3D,
  };
}

export function computeFlatGeometry(geometry, settings) {
  // const theta_min = geometrySettings.thetaMin * Math.PI / 180;
  const thetaMin = (settings.thetaMin === -90) ? -89 * Math.PI / 180 : settings.thetaMin * Math.PI / 180;
  // const theta_max = geometrySettings.thetaMax * Math.PI / 180;
  const thetaMax = (settings.thetaMax === 90) ? 89 * Math.PI / 180 : settings.thetaMin * Math.PI / 180;

  const hTop = (thetaMax <= 0 && settings.hTop === 0) ? 0.0001 : settings.hTop;
  const hBottom = settings.hBottom;

  const { Divisions, divisions, indexWide } = geometry;
  const ellipsoid = geometry.geometry;

  // --------------------------------------------------------------------------
  // Create panel object
  // --------------------------------------------------------------------------

  // step through all thetas starting as the top of the ellipsoid (theta closest to pi/2)
  // at each height (theta) loop through all
  const edgeGeometry = [];
  for (let indexP = 0; indexP < Divisions; indexP += 1) {
    // length of phi is 1 larger than number of edges
    edgeGeometry[indexP] = [];
    for (let indexT = 0; indexT <= divisions; indexT += 1) {
      edgeGeometry[indexP][indexT] = [
        ellipsoid[indexP][indexT],
        ellipsoid[indexP + 1][indexT], // the phi direction is around the circumference of the ellipsoid (cut by horizontal plane)
      ];
    }
  }

  console.debug('Edge Geometry');
  console.debug(edgeGeometry);

  const [edgeGeometryMin, edgeGeometryMax] = edgeGeometryExtents(edgeGeometry);

  console.debug(`Divisions ${Divisions} divisions ${divisions}`);
  console.debug(`Max z : ${edgeGeometryMax.z} Min z : ${edgeGeometryMin.z}`);

  // step through all the edges and flatten them
  // Flatten panel strips one at a time.  For each panel strip, loop through all the angles (theta)
  // starting at the bottom of the half-ellipsoid (theta = 0) and moving up toward theta=pi/2.
  // At each theta calculate the angle between the current panel and the one above it (wrt z axis).
  // Rotate each point in the panel including and below the current panel by the found angle between planes.

  // --------------------------------------------------------------------------
  // Flatten the edges
  // --------------------------------------------------------------------------
  const edgesFlat = cloneDeep(edgeGeometry);

  switch (settings.projection) {
    case 'spherical':

      for (let indexP = 0; indexP < Divisions; indexP += 1) {
        for (let indexT = 0; indexT < divisions; indexT += 1) {
          // rotation happens around the indexT+1 line
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const topPoint = (indexT === divisions - 1) ? {
            x: 0,
            y: 0,
            z: edgeGeometryMax.z,
          } : edgesFlat[indexP][indexT + 2][1];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(edgesFlat[indexP][indexT + 1][0], edgesFlat[indexP][indexT + 1][1], edgesFlat[indexP][indexT][0], topPoint);

          if (hTop > 0 && indexT === divisions - 2 && thetaMax > 0) {
            rotationAngle = -rotationAngle;
          }
          if (hTop > 0 && indexT === divisions - 1) {
            // if hTop is used then the last angle of rotation is 90deg
            rotationAngle = Math.PI / 2;
          }
          // if this is a case where all the thetas are negative and its the "top" of the panel then rotate by
          // the obtuse angle between the planes not the acute angle as determined by angleBetweenPlanes
          if (hBottom > 0 && indexT === 0 && thetaMin < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indexTR = 0; indexTR <= indexT; indexTR += 1) {
            edgesFlat[indexP][indexTR][0] = rotatePoint(edgesFlat[indexP][indexT + 1][1], edgesFlat[indexP][indexT + 1][0], edgesFlat[indexP][indexTR][0], rotationAngle);
            edgesFlat[indexP][indexTR][1] = rotatePoint(edgesFlat[indexP][indexT + 1][1], edgesFlat[indexP][indexT + 1][0], edgesFlat[indexP][indexTR][1], rotationAngle);
          }
        }
      }
      break;
    case 'cylindrical':
      // 1 - figure out the widest area of the ellipsoid
      // hInsert // name of height location (bottom, middle, top)
      // indexh // index location of added height
      // indexWide // index location of widest point on surface

      // 2 - unwrap each panel around the widest part of the cylinder
      for (let indexP = 0; indexP < Divisions; indexP += 1) {
        for (let indexT = 0; indexT < indexWide; indexT += 1) {
          // rotation happens around the indexT+1 line
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const topPoint = (indexT === divisions - 1) ? {
            x: 0,
            y: 0,
            z: edgeGeometryMax.z,
          } : edgeGeometry[indexP][indexT + 2][1];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(edgeGeometry[indexP][indexT + 1][0], edgeGeometry[indexP][indexT + 1][1], edgeGeometry[indexP][indexT][0], topPoint);

          if (hTop > 0 && indexT === divisions && thetaMax > 0) {
            rotationAngle = -rotationAngle;
          }

          // if this is a case where all the thetas are negative and its the "top" of the panel then rotate by
          // the obtuse angle between the planes not the acute angle as determined by angleBetweenPlanes
          if (hBottom > 0 && indexT === 0 && thetaMin < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indexTR = 0; indexTR <= indexT; indexTR += 1) {
            edgesFlat[indexP][indexTR][0] = rotatePoint(edgeGeometry[indexP][indexT + 1][1], edgeGeometry[indexP][indexT + 1][0], edgesFlat[indexP][indexTR][0], rotationAngle);
            edgesFlat[indexP][indexTR][1] = rotatePoint(edgeGeometry[indexP][indexT + 1][1], edgeGeometry[indexP][indexT + 1][0], edgesFlat[indexP][indexTR][1], rotationAngle);
          }
        }

        for (let indexT = divisions; indexT > indexWide + 1; indexT -= 1) {
          // get the point above the rotation line to use to find the angles
          // when we get to the top of the ellipsoid hard code the location
          const bottomPoint = edgeGeometry[indexP][indexT - 2][0];

          // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
          let rotationAngle = angleBetweenPlanes(edgeGeometry[indexP][indexT - 1][0], edgeGeometry[indexP][indexT - 1][1], bottomPoint, edgeGeometry[indexP][indexT][0]);

          if (hTop > 0 && indexT === divisions && thetaMax > 0) {
            rotationAngle = -rotationAngle;
          }
          if (hBottom > 0 && indexT === 0 && thetaMin < 0) {
            rotationAngle = -rotationAngle;
          }

          // loop through all points from this theta to the first theta and rotate them about the current axis
          // This has the effect of uncurling the panel strip a small amount each time we move up the side of the ellipsoid
          // Both points that define each panel edge get rotated (hense the [0] and [1] lines that are otherwise identical)
          for (let indexTR = divisions; indexTR >= indexT; indexTR -= 1) {
            edgesFlat[indexP][indexTR][0] = rotatePoint(edgeGeometry[indexP][indexT - 1][0], edgeGeometry[indexP][indexT - 1][1], edgesFlat[indexP][indexTR][0], rotationAngle);
            edgesFlat[indexP][indexTR][1] = rotatePoint(edgeGeometry[indexP][indexT - 1][0], edgeGeometry[indexP][indexT - 1][1], edgesFlat[indexP][indexTR][1], rotationAngle);
          }
        }
      }

      // 3 - unwrap the cylinder
      for (let indexP = 0; indexP < Divisions; indexP += 1) {
        const prevPanel = (indexP === 0) ? (Divisions - 1) : (indexP - 1);

        // find angle of rotation.  this is the difference in angle from the prev panel to the current panel
        let rotationAngle = angleBetweenPlanes(edgesFlat[indexP][indexWide][0], edgesFlat[indexP][indexWide + 1][0], edgesFlat[indexP][indexWide][1], edgesFlat[prevPanel][indexWide][0]);
        if (indexP === 0) {
          // since phi always starts at 0 we know that we need to rotate half the angle betwee the planes on the first panel.
          // this moves the flattened pattern points to a plane along the min x value (where phi=0)
          rotationAngle /= 2;
        }

        for (let indexPR = indexP; indexPR < Divisions; indexPR += 1) {
          for (let indexT = 0; indexT <= divisions; indexT += 1) {
            edgesFlat[indexPR][indexT][0] = rotatePoint(edgesFlat[indexP][indexWide + 1][0], edgesFlat[indexP][indexWide][0], edgesFlat[indexPR][indexT][0], rotationAngle);
            edgesFlat[indexPR][indexT][1] = rotatePoint(edgesFlat[indexP][indexWide + 1][0], edgesFlat[indexP][indexWide][0], edgesFlat[indexPR][indexT][1], rotationAngle);
          }
        }
      }
      break;

    default:
      console.error('ERROR - Projection Type');
  }

  console.debug('Edges Flattened');
  console.debug(edgesFlat);


  switch (settings.projection) {
    case 'spherical':
      return { edgesFlat: edgesFlat, edges: edgeGeometry, indexWide };
    case 'cylindrical':
      // reorder the points of the flattened edges to be in the x/y domain
      const output = Array.from(edgesFlat, val1 =>
        Array.from(val1, val2 =>
          Array.from(val2, (val3) => {
            return {
              x: val3.y * -1,
              y: val3.z,
              z: val3.x,
            };
          })));
      return { edgesFlat: output, edges: edgeGeometry, indexWide };
    default:
      console.error('ERROR - Projection Type');
  }
}

export function drawEdges(geometrySettings, pattern, scope) {
  const { projection } = geometrySettings;
  const { ppu } = geometrySettings;
  const hTop = geometrySettings.hTop;
  const minGap = geometrySettings.minGap;
  const { imageOffset } = geometrySettings;

  const edgesFlat = cloneDeep(pattern.edgesFlat);

  const { indexWide } = pattern;
  const Divisions = pattern.edgesFlat.length;
  const divisions = pattern.edgesFlat[0].length - 1;

  // calculate a bounding box around the flattened pattern

  const [edgesFlatMin, edgesFlatMax] = edgeGeometryExtents(edgesFlat);

  console.debug(`Flat Pattern Bounding Box ${pointToString(edgesFlatMin)} ${pointToString(edgesFlatMax)}`);

  const image = {
    width: 0,
    height: 0,
  };

  // computer the center point of the image
  const shift = {
    x: 0,
    y: 0,
  };

  image.width = (edgesFlatMax.x - edgesFlatMin.x + 2 * imageOffset) * ppu;
  image.height = (edgesFlatMax.y - edgesFlatMin.y + 2 * imageOffset) * ppu;

  switch (projection) {
    case 'spherical':
      shift.x = (edgesFlatMax.x - edgesFlatMin.x) / 2 + imageOffset;
      shift.y = (edgesFlatMax.y - edgesFlatMin.y) / 2 + imageOffset;
      break;
    case 'cylindrical':
      shift.x = Math.abs(edgesFlatMin.x) + imageOffset;
      shift.y = imageOffset + edgesFlatMax.y;
      break;
    default:
      console.error('ERROR - Projection Type');
  }

  const strokeWidth = 3 * ppu / 90; // enforce that the stroke width be scaled with the units of the drawing

  const patternLayer = scope.project.activeLayer;
  const backgroundLayer = new scope.Layer();
  backgroundLayer.name = 'Bounding Box';
  backgroundLayer.activate();

  const boundingRect = new scope.Shape.Rectangle(new scope.Point(0, 0), new scope.Point(image.width, image.height));
  boundingRect.strokeColor = '#333333';
  boundingRect.strokeWidth = 0.01 * ppu;
  boundingRect.fillColor = new scope.Color(1, 0, 0.5, 0);

  patternLayer.activate();

  // -----------------------------------------------------------------------------
  // assemble ordered points array for the flat pattern
  // -----------------------------------------------------------------------------
  // Loop through all the edges.  For each panel draw the ending edge of the previous panel then
  // the starting edge of the current panel.  This ensures the cutouts are drawn as the outlines as
  // opposed to drawing the edges themselves.
  const pointsFull = [];

  switch (projection) {
    case 'spherical':
      for (let indexP = 0; indexP < Divisions; indexP += 1) {
        // Figure out the index for the previous and the next edges.  This is used to enforce mingap
        const idxPhiPrev = (indexP - 1 < 0) ? Divisions - 1 : indexP - 1;

        for (let indexT = 0; indexT <= divisions; indexT += 1) {
          if (hTop > 0 && indexT === divisions) {
            // the mingap check fails if there is added height to the ellipsoid on top so a special case is needed
            pointsFull.push([shift.x + edgesFlat[idxPhiPrev][indexT][1].x, shift.y + edgesFlat[idxPhiPrev][indexT][1].y]);
          } else if (distance(edgesFlat[idxPhiPrev][indexT][1], edgesFlat[indexP][indexT][0]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[idxPhiPrev][indexT][1].x, shift.y + edgesFlat[idxPhiPrev][indexT][1].y]);
          }
        }
        for (let indexT = divisions; indexT >= 0; indexT -= 1) {
          if (hTop > 0 && indexT === divisions) {
            // the mingap check fails if there is added height to the ellipsoid on top so a special case is needed
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][0].x, shift.y + edgesFlat[indexP][indexT][0].y]);
          } else if (distance(edgesFlat[idxPhiPrev][indexT][1], edgesFlat[indexP][indexT][0]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][0].x, shift.y + edgesFlat[indexP][indexT][0].y]);
          }
        }
      }
      break;
    case 'cylindrical':
      console.debug(`wide ${indexWide}`);
      // let count = 0;

      for (let indexP = 0; indexP < Divisions; indexP += 1) {
        const idxPhiPrev = indexP === 0 ? Divisions - 1 : indexP - 1;
        const idxPhiNext = indexP + 1 === Divisions ? 0 : indexP + 1;

        for (let indexT = indexWide; indexT >= 0; indexT -= 1) {
          if (distance(edgesFlat[idxPhiPrev][indexT][1], edgesFlat[indexP][indexT][0]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][0].x, shift.y - edgesFlat[indexP][indexT][0].y]);
            // s.text((shift.x + edgesFlat[indexP][indexT][0].y)*ppu, (shift.y + edgesFlatMax.z-edgesFlat[indexP][indexT][0].z)*ppu,"b "+indexP+" "+indexT+" "+count).attr({'fill' : 'green',  'stroke': 'green'});
            // count += 1;
          }
        }
        for (let indexT = 0; indexT <= indexWide; indexT += 1) {
          if (distance(edgesFlat[idxPhiNext][indexT][0], edgesFlat[indexP][indexT][1]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][1].x, shift.y - edgesFlat[indexP][indexT][1].y]);
            // s.text((shift.x + edgesFlat[indexP][indexT][1].y)*ppu, (shift.y + edgesFlatMax.z-edgesFlat[indexP][indexT][1].z)*ppu,"d "+indexP+" "+indexT+" "+count).attr({'fill' : 'yellow',  'stroke': 'yellow'});
            // count += 1;
          }
        }
      }
      for (let indexP = Divisions - 1; indexP >= 0; indexP -= 1) {
        const idxPhiPrev = indexP + 1 === Divisions ? 0 : indexP + 1;
        const idxPhiNext = indexP === 0 ? Divisions - 1 : indexP - 1;

        for (let indexT = indexWide; indexT <= divisions; indexT += 1) {
          if (distance(edgesFlat[idxPhiPrev][indexT][0], edgesFlat[indexP][indexT][1]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][1].x, shift.y - edgesFlat[indexP][indexT][1].y]);
            // s.text((shift.x + edgesFlat[indexP][indexT][1].y)*ppu, (shift.y + edgesFlatMax.z-edgesFlat[indexP][indexT][1].z)*ppu,"f "+indexP+" "+indexT+" "+count).attr({'fill' : 'pink',  'stroke': 'pink'});
            // count += 1;
          }
        }
        for (let indexT = divisions; indexT > indexWide; indexT -= 1) {
          if (distance(edgesFlat[idxPhiNext][indexT][1], edgesFlat[indexP][indexT][0]) > minGap) {
            // Enforce minimum gap
            pointsFull.push([shift.x + edgesFlat[indexP][indexT][0].x, shift.y - edgesFlat[indexP][indexT][0].y]);
            // s.text((shift.x + edgesFlat[indexP][indexT][0].y)*ppu, (shift.y + edgesFlatMax.z-edgesFlat[indexP][indexT][0].z)*ppu,"h "+indexP+" "+indexT+" "+count).attr({'fill' : 'orange',  'stroke': 'orange'});
            // count += 1;
          }
        }
      }

      break;
    default:
      console.error('ERROR - Projection Type');
  }


  // -----------------------------------------------------------------------------
  // draw the flat pattern (assemble path point string for cutout)
  // -----------------------------------------------------------------------------
  let pointString = '';

  pointsFull.forEach((point, idx) => {
    let command = '';
    if (idx === 0) {
      command = 'M';
    } else {
      command = 'L';
    }
    pointString += command + (point[0] * ppu).toFixed(3).toString() + ',' + (point[1] * ppu).toFixed(3).toString() + ' ';
  });
  pointString += ' z';
  // add path to drawing

  patternLayer.activate(); // ensure the pattern layer is active
  const path = new scope.Path(pointString);
  path.strokeColor = new scope.Color(0, 0, 0);
  path.strokeWidth = strokeWidth * 0.5;
  path.fillColor = new scope.Color(1, 1, 1, 0.3);


  // -----------------------------------------------------------------------------
  // draw glueing guide lines on the pattern
  // -----------------------------------------------------------------------------
  const guideLineLayer = new scope.Layer();
  guideLineLayer.name = 'Guide Lines';
  guideLineLayer.activate();
  const group = new scope.Group();
  for (let indexP = 0; indexP < Divisions; indexP += 1) {
    for (let indexT = 1; indexT <= divisions; indexT += 1) {
      const line = new scope.Path({
        strokeColor: new scope.Color(0, 1, 0),
        strokeWidth: strokeWidth * 0.25,
      });
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu));
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu));
      group.addChild(line);
    }
  }

  // -----------------------------------------------------------------------------
  // draw pattern clip quadrilaterals
  // -----------------------------------------------------------------------------
  const quadLayer = new scope.Layer();
  quadLayer.name = 'Edges Destination Quadrilaterals';
  quadLayer.activate();
  for (let indexP = 0; indexP < Divisions; indexP += 1) {
    for (let indexT = 0; indexT < divisions; indexT += 1) {
      const line = new scope.Path({
        strokeColor: new scope.Color(1, 0, 0),
        strokeWidth: strokeWidth * 0.25,
      });
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu));
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu));
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu));
      line.add(new scope.Point((shift.x + edgesFlat[indexP][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu));
      line.closed = true;
    }
  }

  // -----------------------------------------------------------------------------
  // draw source pattern quadrilaterals
  // -----------------------------------------------------------------------------
  const quadSourceLayer = new scope.Layer();
  quadSourceLayer.name = 'Edges Source Quadrilaterals';
  quadSourceLayer.activate();

  const leftX = edgesFlat[0][indexWide][0].x;
  const rightX = edgesFlat[Divisions - 1][indexWide][1].x;

  for (let indexP = 0; indexP < Divisions; indexP += 1) {
    for (let indexT = 0; indexT < divisions; indexT += 1) {
      const line = new scope.Path({
        strokeColor: new scope.Color(1, 0, 1),
        strokeWidth: strokeWidth * 0.25,
      });

      const P1a = new scope.Point();
      const P1b = new scope.Point();
      const P2a = new scope.Point();
      const P2b = new scope.Point();
      const P3a = new scope.Point();
      const P3b = new scope.Point();
      const P4a = new scope.Point();
      const P4b = new scope.Point();

      // Since the first and last edges (Divisions) connect to eachother and are always symmetrical, the pattern mapping along
      // the left and right edges will be a vertical line.  The if block below enforces that symmetry.
      if (indexP === 0) {
        if (projection === 'spherical') {
          P1a.set((shift.x + edgesFlat[indexP][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu);
          P1b.set((shift.x + edgesFlat[Divisions - 1][indexT][1].x) * ppu, (shift.y - edgesFlat[Divisions - 1][indexT][1].y) * ppu);
          P2a.set((shift.x + edgesFlat[indexP][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu);
          P2b.set((shift.x + edgesFlat[Divisions - 1][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[Divisions - 1][indexT + 1][1].y) * ppu);
        } else {
          P1a.set((shift.x + leftX) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu);
          P1b.set((shift.x + leftX) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu);
          P2a.set((shift.x + leftX) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu);
          P2b.set((shift.x + leftX) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu);
        }
        P3a.set((shift.x + edgesFlat[indexP][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu);
        P3b.set((shift.x + edgesFlat[indexP + 1][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP + 1][indexT + 1][0].y) * ppu);
        P4a.set((shift.x + edgesFlat[indexP][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu);
        P4b.set((shift.x + edgesFlat[indexP + 1][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP + 1][indexT][0].y) * ppu);
      } else if (indexP === Divisions - 1) {
        P1a.set((shift.x + edgesFlat[indexP][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu);
        P1b.set((shift.x + edgesFlat[indexP - 1][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP - 1][indexT][1].y) * ppu);
        P2a.set((shift.x + edgesFlat[indexP][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu);
        P2b.set((shift.x + edgesFlat[indexP - 1][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP - 1][indexT + 1][1].y) * ppu);
        if (projection === 'spherical') {
          P3a.set((shift.x + edgesFlat[indexP][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu);
          P3b.set((shift.x + edgesFlat[0][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[0][indexT + 1][0].y) * ppu);
          P4a.set((shift.x + edgesFlat[indexP][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu);
          P4b.set((shift.x + edgesFlat[0][indexT][0].x) * ppu, (shift.y - edgesFlat[0][indexT][0].y) * ppu);
        } else {
          P3a.set((shift.x + rightX) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu);
          P3b.set((shift.x + rightX) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu);
          P4a.set((shift.x + rightX) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu);
          P4b.set((shift.x + rightX) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu);
        }
      } else {
        P1a.set((shift.x + edgesFlat[indexP][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT][0].y) * ppu);
        P1b.set((shift.x + edgesFlat[indexP - 1][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP - 1][indexT][1].y) * ppu);
        P2a.set((shift.x + edgesFlat[indexP][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][0].y) * ppu);
        P2b.set((shift.x + edgesFlat[indexP - 1][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP - 1][indexT + 1][1].y) * ppu);
        P3a.set((shift.x + edgesFlat[indexP][indexT + 1][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT + 1][1].y) * ppu);
        P3b.set((shift.x + edgesFlat[indexP + 1][indexT + 1][0].x) * ppu, (shift.y - edgesFlat[indexP + 1][indexT + 1][0].y) * ppu);
        P4a.set((shift.x + edgesFlat[indexP][indexT][1].x) * ppu, (shift.y - edgesFlat[indexP][indexT][1].y) * ppu);
        P4b.set((shift.x + edgesFlat[indexP + 1][indexT][0].x) * ppu, (shift.y - edgesFlat[indexP + 1][indexT][0].y) * ppu);
      }

      line.add(averagePoints(P1a, P1b));
      line.add(averagePoints(P2a, P2b));
      line.add(averagePoints(P3a, P3b));
      line.add(averagePoints(P4a, P4b));
      line.closed = true;
    }
  }
  patternLayer.activate();
}

export function drawNotes(scope, props) {
  const notesLayer = new scope.Layer();
  notesLayer.name = 'Notes';
  notesLayer.activate();

  const units = getUnits(props.ppu);

  const filename = `ellipsoid_a${props.a}${units}_b${props.b}${units}_c${props.c}${units}.svg`;

  const textFilename = new scope.PointText({
    point: [0, 0],
    content: filename,
    fillColor: 'black',
    fontFamily: 'Roboto',
    fontSize: 0.25 * props.ppu,
  });

  textFilename.rotate(-90, textFilename.bounds.bottomRight);
  textFilename.position.x = 0.15 * props.ppu;
  textFilename.position.y = scope.project.layers['Bounding Box'].bounds.height - textFilename.bounds.height / 2 - 0.15 * props.ppu;

  new scope.PointText({
    point: [0.1 * props.ppu, 0.15 * props.ppu],
    content: JSON.stringify(props.input),
    fillColor: 'black',
    fontFamily: 'Courier New',
    fontSize: 0.2 * props.ppu,
  });

  // Draw a ruler on the bottom of the pattern based on the units specified

  const path = new scope.Path();
  // Give the stroke a color
  path.strokeColor = new scope.Color(0.7, 0.3, 0.5);
  path.strokeWidth = 0.01 * props.ppu;
  // var start = new scope.Point(0.1*props.ppu, scope.project.layers['Bounding Box'].bounds.height);
  const start = new scope.Point(scope.project.layers['Ellipsoid Pattern'].bounds.x, scope.project.layers['Bounding Box'].bounds.height);
  // Move to start and draw a line from there
  path.moveTo(start);
  // Note that the plus operator on Point objects does not work
  // in JavaScript. Instead, we need to call the add() function:
  path.lineTo(start.add([0, -0.3 * props.ppu]));

  for (let i = 0; i < scope.project.layers['Ellipsoid Pattern'].bounds.width / props.ppu; i += 1) {
    const copy = path.clone();
    // Distribute the copies horizontally, so we can see them:
    copy.position.x += i * props.input.ppu;
    new scope.PointText({
      point: copy.position,
      content: i + units,
      fillColor: 'black',
      fontFamily: 'Roboto',
      fontSize: 0.2 * props.ppu,
    });
  }
}

export function getUnits(ppu) {
  if (ppu === 96 || ppu === '96') {
    return 'in';
  }
  if (ppu === 3.7795276 || ppu === '3.7795276') {
    return 'mm';
  }
  if (ppu === 37.795276 || ppu === '37.795276') {
    return 'cm';
  }
  return null;
}
