
// -----------------------------------------------------------------------------
// Geometry Functions
// -----------------------------------------------------------------------------

export function distance(P0, P1) {
  // Compute the distance between two 3D points
  const dX = P1.x - P0.x;
  const dY = P1.y - P0.y;
  const dZ = P1.z - P0.z;
  return Math.sqrt(dX * dX + dY * dY + dZ * dZ);
}

export function pointToString(P, n) {
  // Return pretty string version of a point
  // P : point in the form {x:0, y:2, z:4}
  // n : number of decimal places to show
  n = n || 3;
  if ('z' in P) {
    return (
      '(' + P.x.toFixed(n) + ', ' + P.y.toFixed(n) + ', ' + P.z.toFixed(n) + ')'
    );
  }
  return '(' + P.x.toFixed(n) + ', ' + P.y.toFixed(n) + ')';
}

export function rotatePoint(p1, p2, p0, theta) {
  // Rotate a point p0 about a line defined by p1 and p2 by an angle of theta
  // Note that the order of p1 and p2 along with the sign of theta determines the direction of the rotation
  //
  // Return a point rotated about an arbitrary axis in 3D.
  //   Positive angles are counter-clockwise looking down the axis toward the origin.
  //   The coordinate system is assumed to be right-hand.
  //   Arguments: 'axis point 1', 'axis point 2', 'point to be rotated', 'angle of rotation (in radians)'
  //
  // Primary Reference :
  // http://paulbourke.net/geometry/rotate/
  // Algorithm adapted for JS from http://paulbourke.net/geometry/rotate/PointRotate.py
  // Additional Reference :
  // https://sites.google.com/site/glennmurray/Home/rotation-matrices-and-formulas/rotation-about-an-arbitrary-axis-in-3-dimensions
  // https://en.wikipedia.org/wiki/Rotation_matrix#Rotation_matrix_from_axis_and_angle

  // p = p0 - p1
  const p = {
    x: p0.x - p1.x,
    y: p0.y - p1.y,
    z: p0.z - p1.z,
  };

  // Initialize point q
  const q = {
    x: 0,
    y: 0,
    z: 0,
  };
  // N = (p2 - p1)
  const N = {
    x: p2.x - p1.x,
    y: p2.y - p1.y,
    z: p2.z - p1.z,
  };
  const Nm = Math.sqrt(N.x * N.x + N.y * N.y + N.z * N.z);

  // # Rotation axis unit vector
  const n = {
    x: N.x / Nm,
    y: N.y / Nm,
    z: N.z / Nm,
  };

  // # Matrix common factors
  const c = Math.cos(theta);
  const t = 1 - Math.cos(theta);
  const s = Math.sin(theta);
  const X = n.x;
  const Y = n.y;
  const Z = n.z;

  // # Matrix 'M'
  const d11 = t * X * X + c;
  const d12 = t * X * Y - s * Z;
  const d13 = t * X * Z + s * Y;
  const d21 = t * X * Y + s * Z;
  const d22 = t * Y * Y + c;
  const d23 = t * Y * Z - s * X;
  const d31 = t * X * Z - s * Y;
  const d32 = t * Y * Z + s * X;
  const d33 = t * Z * Z + c;

  // # | p.x | #Matrix 'M' * | p.y | # | p.z |
  q.x = d11 * p.x + d12 * p.y + d13 * p.z;
  q.y = d21 * p.x + d22 * p.y + d23 * p.z;
  q.z = d31 * p.x + d32 * p.y + d33 * p.z;

  // # Translate axis and rotated point back to original location
  return {
    x: q.x + p1.x,
    y: q.y + p1.y,
    z: q.z + p1.z,
  };
}


// give plane normal based on three points that define the plane
export function planeNormal(A, B, C) {
  const v1 = {
    x: B.x - A.x,
    y: B.y - A.y,
    z: B.z - A.z,
  };
  const v2 = {
    x: C.x - A.x,
    y: C.y - A.y,
    z: C.z - A.z,
  };
  return {
    x: v1.y * v2.z - v1.z * v2.y,
    y: v1.z * v2.x - v1.x * v2.z,
    z: v1.x * v2.y - v1.y * v2.x,
  };
}

// find angle between two planes defined by 4 points
// Points A and B are on both planes
// https://math.tutorvista.com/geometry/angle-between-two-planes.html
export function angleBetweenPlanes(A, B, C, D) {
  // get normals for both planes
  const N1 = planeNormal(A, B, C);
  const N2 = planeNormal(A, B, D);

  return Math.acos(Math.abs(N1.x * N2.x + N1.y * N2.y + N1.z * N2.z) / (Math.sqrt(N1.x * N1.x + N1.y * N1.y + N1.z * N1.z) * Math.sqrt(N2.x * N2.x + N2.y * N2.y + N2.z * N2.z)));
}
