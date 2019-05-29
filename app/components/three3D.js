import React, { Component } from 'react';
import { connect } from 'react-redux';
import * as THREE from 'three';
import {OBJLoader} from 'three/examples/jsm/loaders/OBJLoader'; //https://threejs.org/docs/#manual/en/introduction/Import-via-modules
import {OrbitControls} from 'three/examples/jsm/controls/OrbitControls';

import {
  getUnits,
} from '../utils/ellipsoid';

class Three3D extends Component {
  constructor(props) {
    super(props);
    this.handleDownload = this.handleDownload.bind(this);
    this.animate = this.animate.bind(this);
    this.updateMesh = this.updateMesh.bind(this);
  }

  componentDidMount() {
    const width = this.mount.clientWidth;;
    const height = this.mount.clientHeight;

    this.scene = new THREE.Scene();
    //this.camera = new THREE.PerspectiveCamera(75, width / height, 0.1, 4000);
    this.camera = new THREE.OrthographicCamera( width / -2, width / 2, height / 2, height / -2, 1, 2000 );
    this.camera.up.set( 0, 0, 1 );
    this.camera.position.x = 7;
    this.camera.position.y = 7;
    this.camera.position.z = 2;

    this.renderer = new THREE.WebGLRenderer( { antialias: true, preserveDrawingBuffer: true} );
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap; // default THREE.PCFShadowMap
    this.renderer.gammaInput = true;
    this.renderer.gammaOutput = true;
    this.renderer.setSize(width, height);
    this.renderer.setClearColor( 0xbbbbbb, 1.0 );

    this.renderer.setViewport(0, 0, width, height);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.target.set(0, 0, 0);
    this.controls.rotateSpeed = 1.0;
    this.controls.zoomSpeed = 1.2;
    this.controls.panSpeed = 0.8;
    this.controls.update();

    this.mount.appendChild(this.renderer.domElement);

    // Light
    // var pointLight = new THREE.PointLight( 0xFFFFFF, 2);
    // pointLight.position.set( 200, 250, 600 );
    // pointLight.castShadow = true;
    // scene.add( pointLight );
    const spotLight = new THREE.SpotLight( 0xFFFFFF, 1);
    spotLight.position.set( 0, 250, 50 );
    spotLight.target.position.set( 0, 0, 0 );
    // spotLight.castShadow = true;
    // //Set up shadow properties for the spotLight
    // spotLight.shadow.mapSize.width = 512;  // default
    // spotLight.shadow.mapSize.height = 512; // default
    // spotLight.shadow.camera.near = 0.5;    // default
    // spotLight.shadow.camera.far = 1500;     // default
    this.scene.add( spotLight.target );
    this.scene.add( spotLight );

    const spotLight2 = new THREE.SpotLight( 0xFFFFFF, 1);
    spotLight2.position.set( -100, -100, -25 );
    spotLight2.target.position.set( 0, 0, 0 );
    // spotLight2.castShadow = true;
    // //Set up shadow properties for the spotLight
    // spotLight2.shadow.mapSize.width = 512;  // default
    // spotLight2.shadow.mapSize.height = 512; // default
    // spotLight2.shadow.camera.near = 0.5;    // default
    // spotLight2.shadow.camera.far = 1500;     // default
    this.scene.add( spotLight2.target );
    this.scene.add( spotLight2 );

    const spotLight3 = new THREE.SpotLight( 0xFFFFFF, 1);
    spotLight3.position.set( 100, -100, -25 );
    spotLight3.target.position.set( 0, 0, 0 );
    // spotLight3.castShadow = true;
    // //Set up shadow properties for the spotLight
    // spotLight3.shadow.mapSize.width = 512;  // default
    // spotLight3.shadow.mapSize.height = 512; // default
    // spotLight3.shadow.camera.near = 0.5;    // default
    // spotLight3.shadow.camera.far = 1500;     // default
    this.scene.add( spotLight3.target );
    this.scene.add( spotLight3 );

    // var directionalLight = new THREE.DirectionalLight( 0xFFFFFF, 1 );
    // directionalLight.position.set( 100, 350, 250 );
    // directionalLight.castShadow = true;
    // this.scene.add( directionalLight );

    // this.scene.fog = new THREE.FogExp2(0xFFFFFF, 0.05);

    const ambientLight = new THREE.AmbientLight( 0x404040 );
    this.scene.add(ambientLight);

    const axesHelper = new THREE.AxesHelper( 10 );
    this.scene.add( axesHelper );

    const material = new THREE.MeshStandardMaterial({ color: 0x0087E6});
    material.side = THREE.DoubleSide;



    // console.log('          CUBE');
    // const geometry = new THREE.BoxGeometry( 2, 5, 6 );
    // console.log(geometry);
    // let cube = new THREE.Mesh( geometry, material );
    // cube.position.set(0, 1, 1);
    // cube.rotation.x = -30 * Math.PI/180;
    // cube.castShadow = true;
    // console.log(cube);
    // this.scene.add( cube );

    // // Create a plane that receives shadows (but does not cast them)
    // const planeGeometry = new THREE.PlaneBufferGeometry( 10000, 10000, 32, 32 );
    // const planeMaterial = new THREE.MeshLambertMaterial( { color: 0xb69a77, side: THREE.DoubleSide } );
    // const plane = new THREE.Mesh( planeGeometry, planeMaterial );
    // plane.receiveShadow = true;
    // //plane.rotation.x = - Math.PI / 2;
    // plane.position.z = -3;
    // this.scene.add( plane );

    // populate scene with a mesh called ellipsoid (this gets removed when the actual ellipsoid is loaded)
    const objTemp = "v  1.000000 -1.000000 -1.000000\n    v  1.000000 -1.000000  1.000000\n    v -1.000000 -1.000000  1.000000\n    v -1.000000 -1.000000 -1.000000\n    v  1.000000  1.000000 -1.000000\n    v  1.000000  1.000000  1.000000\n    v -1.000000  1.000000  1.000000\n    v -1.000000  1.000000 -1.000000\n    s off\n    f 1 2 3 4\n    f 5 6 7 8\n    f 1 4 8 5\n    f 2 3 7 6\n    f 1 2 6 5\n    f 3 4 8 7";
    this.loader = new OBJLoader();
    this.ellipsoid = this.loader.parse(objTemp);
    this.ellipsoid.name = 'ellipsoid';
    this.ellipsoid.traverse( ( child ) => {
      if ( child.isMesh ) {
        child.material = material;
      }
    } );
    this.scene.add(this.ellipsoid);
    console.log(this.ellipsoid);

    this.animate();
  }

  componentDidUpdate() {
    const { obj3D } = this.props;
    
    // update size of window
    const width = this.mount.clientWidth;;
    const height = this.mount.clientHeight;
    const canvas = this.renderer.domElement; 
    canvas.width  = width;
    canvas.height = height;
    canvas.style = {width, height};
    this.renderer.setViewport(0, 0, width, height);

    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();

    // update geometry (3D model)
    this.updateMesh(obj3D);
  }

  componentWillUnmount() {
    cancelAnimationFrame(this.frameId);
    this.mount.removeChild(this.renderer.domElement);
  }

  animate() {
    this.frameId = window.requestAnimationFrame(this.animate);
    this.renderer.render(this.scene, this.camera);
  }

  updateMesh(obj3D) {
    // define material to apply to each mesh panel
    const ellipsoidMaterial = new THREE.MeshStandardMaterial({ color: 0x0087E6});
    ellipsoidMaterial.side = THREE.DoubleSide;

    // remove the previous mesh from the scene
    const selectedObject = this.scene.getObjectByName('ellipsoid');
    this.scene.remove( selectedObject );

    // parse the OBJ mesh
    this.loader = new OBJLoader();
    this.ellipsoid = this.loader.parse(obj3D);
    this.ellipsoid.castShadow = true;
    this.ellipsoid.name = 'ellipsoid';

    // apply material to each panel in the mesh
    this.ellipsoid.traverse( ( child ) => {
      if ( child.isMesh ) {
        child.material = ellipsoidMaterial;
      }
    } );

    // add the mesh to the scene
    this.scene.add(this.ellipsoid);

    // update camera

    // create an helper box around the ellipsoid
    const helper = new THREE.BoxHelper(this.ellipsoid);
    helper.update();
    // get the bounding sphere
    const {center, radius} = helper.geometry.boundingSphere;
    // calculate the height of the ellipsoid
    const realHeight = Math.abs((center.z+radius) - (center.z-radius));

    // update the camera position and frustum to keep the ellipsoid in view
    this.camera.left = realHeight / -2;
    this.camera.right = realHeight / 2;
    this.camera.top = realHeight / 2;
    this.camera.bottom = realHeight / -2;

    this.camera.position.x = radius;
    this.camera.position.y = radius;
    this.camera.position.z = radius*.25;

    this.camera.updateProjectionMatrix();

    this.controls.target.set(center.x, center.y, center.z);


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
    
    // if size is undefined create something small to render to
    let sizeLocal = size;
    if (sizeLocal === 'undefinedpx') {
      sizeLocal = '100px';
    }
    
    if (id === 'edges' ) { // if this is the edges view then show the download button

      // this return should be modified to display the obj instead of using view3D
      // three.js will be used
      //   OBJLoader.parse() will convert the obj string to Object3D
      //   that Object3D can then be put into the scene with scene.add()
      
      return (
        <div>
          <button type="submit" onClick={this.handleDownload}>Download OBJ</button>
          <br />
          <div
            id={id}
            style={{ width: sizeLocal, height: sizeLocal }}
            ref={mount => {
              this.mount = mount;
            }}
          />
        </div>
      );
    }

    return (
      <div>
        <div id={id} />
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

export default connect(mapStateToProps, null)(Three3D);
