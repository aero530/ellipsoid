import React, { Component } from 'react';
import { connect } from 'react-redux';
import * as vis from 'vis';

function drawVisualization(edges, elementid) {
  // Function to generate 3D plot of edges

  // create the data table.
  const data = new vis.DataSet();

  // track the max value in each axis. used to scale the z axis

  let maxX = 0;
  let maxZ = 0;
  let minZ = 0;

  // create the animation data
  edges.forEach((slice, idx0) => {
    slice.forEach((line) => {
      if (Math.abs(line[0].x) > maxX) {
        maxX = Math.abs(line[0].x);
      } // abs value is used because if the number of phi divisions is odd the "max" extent may be on the negative side
      if (line[0].z > maxZ) {
        maxZ = line[0].z;
      }
      if (line[0].z < minZ) {
        minZ = line[0].z;
      }
      if (Math.abs(line[1].x) > maxX) {
        maxX = Math.abs(line[1].x);
      }
      if (line[1].z > maxZ) {
        maxZ = line[1].z;
      }
      if (line[1].z < minZ) {
        minZ = line[1].z;
      }
      // minZ = (line[1].z < minZ) ? line[1].z : minZ;
      data.add({
        x: line[0].x,
        y: line[0].y,
        z: line[0].z,
        style: parseInt(idx0, 10), // color the dots by the panel column they are in
      });
      data.add({
        x: line[1].x,
        y: line[1].y,
        z: line[1].z,
        style: parseInt(idx0, 10),
      });
    });
  });

  // specify options
  const options = {
    width: '100%',
    height: '100%',
    style: 'dot-color',
    showPerspective: false,
    showGrid: true,
    keepAspectRatio: true,
    verticalRatio: (maxZ - minZ) / maxX * 0.5,
    legendLabel: 'value',
    cameraPosition: {
      horizontal: -0.25,
      vertical: 0.25,
      distance: 1.6,
    },
  };

  // create our graph
  const container = document.getElementById(elementid);
  // eslint-disable-next-line
  var graph = new vis.Graph3d(container, data, options);
}

class View3D extends Component {
  componentDidUpdate() {
    const { shape, id } = this.props;
    drawVisualization(shape, id);
  }

  render() {
    const { size, id } = this.props;
    return (
      <div id={id} style={{ width: size, height: size }} />
    );
  }
}

// connects root reducer to props
function mapStateToProps(state, ownprops) {
  return {
    shape: state.edges[ownprops.id],
  };
}

export default connect(mapStateToProps, null)(View3D);
