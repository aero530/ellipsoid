import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import PropTypes from 'prop-types';
import * as paper from 'paper';

import {computeGeometry, computePattern, drawPattern, drawNotes, getUnits} from './ellipsoid.js';
import {processTexture} from './texture.js';

import {updateGeometry} from '../actions';
import {updatePattern} from '../actions';

class Scene extends Component {
  constructor(props) {
    super(props)
    window.paper = new paper.PaperScope();
    
    this.handleTexture = this.handleTexture.bind(this);
    this.handleDownload = this.handleDownload.bind(this);
    this.handleUpdateGeometry = this.handleUpdateGeometry.bind(this);
    this.handleUpdatePattern = this.handleUpdatePattern.bind(this);
  }
  
  handleTexture() {
    processTexture(this.props.texture, 5, window.paper);
  }

  handleUpdateGeometry(data) {
    this.props.updateGeometry(data);
  }
  handleUpdatePattern(data) {
    this.props.updatePattern(data);
  }

  handleDownload() {
    const scope = window.paper;
    // use export bounds:"content" to export full size image (not scaled down view)
    let svgString = scope.project.exportSVG({bounds:"content", asString:true});
    // add inkscape layer modifier to all groups that have an id defined right after the start of the group element
    svgString = svgString.replace(/<g id="([a-zA-Z0-9\t\n ./<>?;:"'`!@#$%^&*()[\]{}_+=|\\-~,]*)"/g, (match, p1) => {
      return "<g inkscape:groupmode=\"layer\" id=\""+p1+"\"";
    });

    const url = "data:image/svg+xml;utf8," + encodeURIComponent(svgString);
    
    const link = document.createElement("a");

    let units = getUnits(this.props.ppu);
    const filename = "ellipsoid_a" + this.props.a + units +
      "_b" + this.props.b + units +
      "_c" + this.props.c + units +
      ".svg";

    link.download = filename;
    link.href = url;
    link.click();
  }

  handleDrawPattern(pattern) {
    let scope = window.paper;

    if (scope.projects.length === 0) {  // if there is not a project define (first time this is loaded)
      scope.setup(document.getElementById('paper')); // setup the project
    } else {
      scope.project.clear(); // clear the project of all content
    }

    var patternLayer = scope.project.activeLayer;
    patternLayer.name = 'Ellipsoid Pattern';

    drawPattern(this.props.geometrySettings, this.props.projectionSettings, pattern, scope);
    scope.project.layers['Pattern Destination Quadrilaterals'].remove();
    scope.project.layers['Pattern Source Quadrilaterals'].remove();

    // Get the size of the 'Bounding Box' layer and use it to set the size of the image
    let imgWidth = scope.project.layers['Bounding Box'].bounds.width;
    let imgHeight = scope.project.layers['Bounding Box'].bounds.height;

    let viewWidth = scope.view.viewSize.width;
    let viewHeight = scope.view.viewSize.height;

    let zoom = Math.min(viewWidth/imgWidth, viewHeight/imgHeight);
    
    //scope.view.viewSize = new scope.Size(imgWidth,imgHeight);
    scope.view.zoom = zoom;
    // scope.view.viewSize = new scope.Size(imgWidth*zoom,imgHeight*zoom);
    scope.view.center = scope.project.activeLayer.bounds.center;

    // if there is enough of a gap around the image, put some notes there
    if (this.props.imageOffset >= 0.5) {  
      drawNotes(scope, this.props);
    }
  }

  render() {
    return (
      <div>
      {//<button onClick={this.handleTexture}>Texturize</button>
      }
      <button onClick={this.handleDownload}>Download</button>
      <br />
      <canvas id="paper" width={900} height={900} />
      </div>
    )
  }

  componentDidUpdate() {
    const geometry = computeGeometry(this.props.geometrySettings);
    this.handleUpdateGeometry(geometry);

    const pattern = computePattern(geometry, this.props.geometrySettings, this.props.projectionSettings.projection);
    this.handleUpdatePattern(pattern);

    this.handleDrawPattern(pattern);

    this.handleTexture();
  }

  componentDidMount() {
    const geometry = computeGeometry(this.props.geometrySettings);
    this.handleUpdateGeometry(geometry);

    const pattern = computePattern(geometry,  this.props.geometrySettings, this.props.projectionSettings.projection);
    this.handleUpdatePattern(pattern);

    this.handleDrawPattern(pattern);
    this.handleTexture();
  }
}

//connects root reducer to props
const mapStateToProps = state => ({
    geometrySettings: state.form.EllipsoidInput.values,
    projectionSettings: state.form.ProjectionInput.values,
    a: state.form.EllipsoidInput.values.a.toFixed(2).toString(),
    b: state.form.EllipsoidInput.values.b.toFixed(2).toString(),
    c: state.form.EllipsoidInput.values.c.toFixed(2).toString(),
    ppu: state.form.EllipsoidInput.values.ppu,
    imageOffset: 7,//state.form.ProjectionInput.values.imageOffset.toFixed(2),
    texture: state.file
});

//connects redux actions to props
function mapDispatchToProps(dispatch) {
    return bindActionCreators({
        updateGeometry: updateGeometry,
        updatePattern: updatePattern
    }, dispatch);
}

Scene.propTypes = {
    geometrySettings : PropTypes.object.isRequired,
    projectionSettings : PropTypes.object.isRequired,
    a : PropTypes.string.isRequired,
    b : PropTypes.string.isRequired,
    c : PropTypes.string.isRequired,
    ppu : PropTypes.number.isRequired,
    imageOffset : PropTypes.number.isRequired,
    texture : PropTypes.string.isRequired
}

export default connect(mapStateToProps, mapDispatchToProps)(Scene);