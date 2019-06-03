import React, { Component } from 'react';
import compose from 'recompose/compose';
import { connect } from 'react-redux';
import { withStyles } from '@material-ui/core/styles';
import PropTypes from 'prop-types';
import * as paper from 'paper';
import {
  computeGeometry,
  computeFlatGeometry,
  drawEdges,
  drawNotes,
  getUnits,
} from '../utils/ellipsoid';

import { UPDATE_GEOMETRY, UPDATE_EDGES } from '../actions';

const styles = theme => ({
  root: {
    // width: '100%',
    padding: theme.spacing.unit * 2,
  },
  paper: {
    padding: theme.spacing.unit * 2,
  },
});

class Scene extends Component {
  constructor(props) {
    super(props);
    window.paper = new paper.PaperScope();
    this.myRef = React.createRef();

    this.handleDownload = this.handleDownload.bind(this);
    this.handleUpdateGeometry = this.handleUpdateGeometry.bind(this);
    this.handleUpdateEdges = this.handleUpdateEdges.bind(this);
  }

  componentDidMount() {
    const { input } = this.props;
    const geometry = computeGeometry(input);
    this.handleUpdateGeometry(geometry);

    const edges = computeFlatGeometry(geometry, input);
    this.handleUpdateEdges(edges);
    this.handleDrawEdges(edges);
  }

  componentDidUpdate() {
    const { input } = this.props;
    const geometry = computeGeometry(input);
    this.handleUpdateGeometry(geometry);

    const edges = computeFlatGeometry(geometry, input);
    this.handleUpdateEdges(edges);
    this.handleDrawEdges(edges);
    
  }

  // handleTexture() {
  //   const { texture } = this.props;
  //   processTexture(texture, 5, window.paper);
  // }

  onResize = () => {
    const { input } = this.props;
    const geometry = computeGeometry(input);
    this.handleUpdateGeometry(geometry);

    const edges = computeFlatGeometry(geometry, input);
    this.handleUpdateEdges(edges);

    this.handleDrawEdges(edges);
  }

  handleUpdateGeometry(data) {
    this.props.updateGeometry(data);
  }

  handleUpdateEdges(data) {
    this.props.updateEdges(data);
  }

  handleDownload() {
    const scope = window.paper;
    const {
      ppu,
      a,
      b,
      c,
      inkscapeLayers,
    } = this.props;

    // use export bounds:"content" to export full size image (not scaled down view)
    let svgString = scope.project.exportSVG({ bounds: 'content', asString: true });

    if (inkscapeLayers) {
      // add inkscape layer modifier to all groups that have an id defined right after the start of the group element
      svgString = svgString.replace(/<g id="([a-zA-Z0-9\t\n ./<>?;:"'`!@#$%^&*()[\]{}_+=|\\-~,]*)"/g, (match, p1) => `<g inkscape:groupmode="layer" id="${p1}"`);
    }

    const url = `data:image/svg+xml;utf8,${encodeURIComponent(svgString)}`;

    const link = document.createElement('a');

    const units = getUnits(ppu);
    const filename = `ellipsoid_a${a}${units}_b${b}${units}_c${c}${units}.svg`;

    link.download = filename;
    link.href = url;
    link.click();
  }

  handleDrawEdges(edges) {
    const { imageOffset, input } = this.props;
    const scope = window.paper;

    if (scope.projects.length === 0) { // if there is not a project define (first time this is loaded)
      scope.setup(document.getElementById('paper')); // setup the project
    } else {
      scope.project.clear(); // clear the project of all content
      scope.setup(document.getElementById('paper')); // setup the project
    }

    const edgesLayer = scope.project.activeLayer;
    edgesLayer.name = 'Ellipsoid Pattern';

    drawEdges(input, edges, scope);
    scope.project.layers['Edges Destination Quadrilaterals'].remove();
    scope.project.layers['Edges Source Quadrilaterals'].remove();

    // Get the size of the 'Bounding Box' layer and use it to set the size of the image
    const imgWidth = scope.project.layers['Bounding Box'].bounds.width;
    const imgHeight = scope.project.layers['Bounding Box'].bounds.height;

    const viewWidth = scope.view.viewSize.width;
    const viewHeight = scope.view.viewSize.height;

    const zoom = Math.min(viewWidth / imgWidth, viewHeight / imgHeight);

    // scope.view.viewSize = new scope.Size(imgWidth,imgHeight);
    scope.view.zoom = zoom;
    // scope.view.viewSize = new scope.Size(imgWidth*zoom,imgHeight*zoom);
    scope.view.center = scope.project.activeLayer.bounds.center;

    // if there is enough of a gap around the image, put some notes there
    if (imageOffset >= 0.5) {
      drawNotes(scope, this.props);
    }
  }

  render() {
    const { size } = this.props;
    
    // if size is undefined create something small to render to
    let sizeLocal = size;
    if (sizeLocal === 'undefinedpx') {
      sizeLocal = '100px';
    }

    return (
      <div>
        <button type="submit" onClick={this.handleDownload}>Download SVG</button>
          <br />
          <canvas style={{ width: sizeLocal, height: sizeLocal }} id='paper' width={sizeLocal} height={sizeLocal} />
      </div>
    );
  }
}


// connects root reducer to props
const mapStateToProps = state => ({
  input: state.input,
  a: state.input.a.toFixed(2).toString(),
  b: state.input.b.toFixed(2).toString(),
  c: state.input.c.toFixed(2).toString(),
  ppu: state.input.ppu,
  imageOffset: state.input.imageOffset.toFixed(2), // state.form.ProjectionInput.values.imageOffset.toFixed(2),
  inkscapeLayers: state.input.inkscapeLayers,
  // texture: state.file,
});

// connects redux actions to props
const mapDispatchToProps = dispatch => ({
  updateGeometry: (input) => dispatch({ type: UPDATE_GEOMETRY, value: input }),
  updateEdges: (input) => dispatch({ type: UPDATE_EDGES, value: input }),
});

export default compose(
  withStyles(styles),
  connect(
    mapStateToProps,
    mapDispatchToProps,
  ),
)(Scene);
