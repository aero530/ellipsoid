import React, { Component } from 'react';
import {connect} from 'react-redux';
import * as vis from 'vis';

function drawVisualization(panels, elementid) {
  // Function to generate 3D plot of panels

  // create the data table.
  var data = new vis.DataSet();

  // track the max value in each axis. used to scale the z axis

  var maxX = 0;
  var maxZ = 0;
  var minZ = 0;

  // create the animation data
  panels.forEach(function(slice,idx0){
    slice.forEach(function(line,idx1){
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
      //minZ = (line[1].z < minZ) ? line[1].z : minZ;
      data.add({
        x: line[0].x,
        y: line[0].y,
        z: line[0].z,
        style: parseInt(idx0,10) // color the dots by the panel column they are in
      });
      data.add({
        x: line[1].x,
        y: line[1].y,
        z: line[1].z,
        style: parseInt(idx0,10)
      });
    });
  });

  // specify options
  const options = {
    width: "100%",
    height: "100%",
    style: "dot-color",
    showPerspective: false,
    showGrid: true,
    keepAspectRatio: true,
    verticalRatio: (maxZ - minZ) / maxX * 0.5,
    legendLabel: "value",
    cameraPosition: {
      horizontal: -0.25,
      vertical: 0.25,
      distance: 1.6
    }
  };

  // create our graph
  const container = document.getElementById(elementid);
  var graph = new vis.Graph3d(container, data, options);
}

class View3D extends Component {

    render() {
        return (
            <div id={this.props.id} style={{width:this.props.size, height:this.props.size}}/>
        );
    }

    componentDidUpdate() {
        drawVisualization(this.props.shape, this.props.id)
    }

}

//connects root reducer to props
function mapStateToProps(state, ownprops) {
    return {
        id: ownprops.geometry,
        size: ownprops.size,
        shape: state.ellipsoid[ownprops.geometry]
    }
}
  
export default connect(mapStateToProps, null)(View3D);