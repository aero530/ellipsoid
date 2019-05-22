import React, { Component } from 'react';
import { connect } from 'react-redux';
import * as vis from 'vis';
import ReactResizeDetector  from 'react-resize-detector';
import * as THREE from 'three';
import {OBJLoader} from 'three/examples/jsm/loaders/OBJLoader'; //https://threejs.org/docs/#manual/en/introduction/Import-via-modules

import {
  getUnits,
} from '../utils/ellipsoid';

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

function drawObj(objString) {

  const width = 200;
  const height = 200;
  // ADD SCENE
  const scene = new THREE.Scene()
  // ADD CAMERA
  const camera = new THREE.PerspectiveCamera(
    75,
    width / height,
    0.1,
    1000
  )
  camera.position.z = 4
  // ADD RENDERER
  const objCanvas = document.getElementById('objscene');
  const renderer = new THREE.WebGLRenderer({ antialias: true, canvas: objCanvas })
  renderer.setClearColor('#000000')
  renderer.setSize(width, height)

  //this.mount.appendChild(renderer.domElement)
  //document.body.appendChild( renderer.domElement );

  // ADD CUBE
  const geometry = new THREE.BoxGeometry(1, 1, 1)
  const material = new THREE.MeshBasicMaterial({color: '#433F81'})
  const cube = new THREE.Mesh(geometry, material)
  const loader = new OBJLoader;
  
  const ellipsoid = loader.parse(objString);
  console.log("cube");
  console.log(cube);
  console.log("ellipsoid");
  console.log(ellipsoid);
  scene.add(cube)
  scene.add(ellipsoid)
  renderer.render(scene, camera);
}

class View3D extends Component {
  constructor(props) {
    super(props);
    this.handleDownload = this.handleDownload.bind(this);
  }

  componentDidUpdate() {
    const { shape, id, obj3D } = this.props;
    drawVisualization(shape, id);
    drawObj(obj3D);
  }

  handleDownload() {
    const {
      a,
      b,
      c,
      ppu,
      obj3D,
    } = this.props;

    const url = `data:text/plain;utf8,${encodeURIComponent(obj3D)}`;
    const link = document.createElement('a');
    const units = getUnits(ppu);
    const filename = `ellipsoid_a${a}${units}_b${b}${units}_c${c}${units}.obj`;

    link.download = filename;
    link.href = url;
    link.click();
  }

  render() {
    const { size, id } = this.props;
    
    if (id === 'edges') { // if this is the edges view then show the download button

      // this return should be modified to display the obj instead of using view3D
      // three.js will be used
      //   OBJLoader.parse() will convert the obj string to Object3D
      //   that Object3D can then be put into the scene with scene.add()

      return (
        <div>
          <button type="submit" onClick={this.handleDownload}>Download OBJ</button>
          <br />
          <div id={id} style={{ width: size, height: size }} />
          <ReactResizeDetector handleWidth handleHeight onResize={this.onResize}>
            {(width) => {
              return(<canvas id="objscene" width={width} height={width} />);
            }}
          </ReactResizeDetector>
        </div>
      );
    }

    return (
      <div>
        <div id={id} style={{ width: size, height: size }} />
      </div>
    );
  }
}

// connects root reducer to props
function mapStateToProps(state, ownprops) {
  return {
    shape: state.edges[ownprops.id],
    obj3D: state.geometry.obj,
    a: state.input.a.toFixed(2).toString(),
    b: state.input.b.toFixed(2).toString(),
    c: state.input.c.toFixed(2).toString(),
    ppu: state.input.ppu,
  };
}

export default connect(mapStateToProps, null)(View3D);
