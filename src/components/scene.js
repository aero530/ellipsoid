import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import * as paper from 'paper';
import {computeGeometry, computePattern, drawPattern} from './ellipsoid.js';

import {updateGeometry} from '../actions';
import {updatePattern} from '../actions';

class Scene extends Component {
    constructor(props) {
        super(props)
        window.paper = new paper.PaperScope();
        this.handleDownload = this.handleDownload.bind(this);
        this.handleUpdateGeometry = this.handleUpdateGeometry.bind(this);
        this.handleUpdatePattern = this.handleUpdatePattern.bind(this);
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

        let units = "";

        if (this.props.ppu === 96) {
            units = "in";
        } else if (this.props.ppu === 3.7795276) {
            units = "mm";
        } else if (this.props.ppu === 37.795276) {
            units = "cm";
        }

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

        console.log(scope);
        // var firstLayer = new scope.Layer();
        // firstLayer.name = 'Layer ABC';
        // firstLayer.activate();

        // var path = new scope.Path();
		// // Give the stroke a color
		// path.strokeColor = new scope.Color(this.props.color[0]/255, this.props.color[1]/255, this.props.color[2]/255);
		// var start = new scope.Point(0, 0);
		// // Move to start and draw a line from there
		// path.moveTo(start);
		// // Note that the plus operator on Point objects does not work
		// // in JavaScript. Instead, we need to call the add() function:
        // path.lineTo(start.add([ 200, 500 ]));
        
        // var secondLayer = new scope.Layer();
        // secondLayer.name = 'Layer 559';
        // secondLayer.activate();

        // var circle = new scope.Shape.Circle(new scope.Point(50,75), 30);
        // circle.strokeColor = "#333333";
        // circle.fillColor = "#0084B0";

        // firstLayer.activate();
        // var ellipse = new scope.Shape.Ellipse(new scope.Point(20,100), new scope.Size(100, 20));
        // ellipse.fillColor = new scope.Color(this.props.color[0]/255, this.props.color[1]/255, this.props.color[2]/255);

    }
    
    render() {
        return (
            <div>
            <canvas id="paper" width={900} height={900} />
            <button onClick={this.handleDownload}>Download</button>
            </div>
        )
    }

    componentDidUpdate() {
        const geometry = computeGeometry(this.props.geometrySettings);
        this.handleUpdateGeometry(geometry);

        const pattern = computePattern(geometry, this.props.geometrySettings, this.props.projectionSettings.projection);
        this.handleUpdatePattern(pattern);

        this.handleDrawPattern(pattern);
    }

    componentDidMount() {
        const geometry = computeGeometry(this.props.geometrySettings);
        this.handleUpdateGeometry(geometry);

        const pattern = computePattern(geometry,  this.props.geometrySettings, this.props.projectionSettings.projection);
        this.handleUpdatePattern(pattern);

        this.handleDrawPattern(pattern);
    }
}

//connects root reducer to props
function mapStateToProps(state) {
    return {
      //color: state.color,
      geometrySettings: state.form.EllipsoidInput.values,
      projectionSettings: state.form.ProjectionInput.values,
      a: state.form.EllipsoidInput.values.a.toFixed(2).toString(),
      b: state.form.EllipsoidInput.values.b.toFixed(2).toString(),
      c: state.form.EllipsoidInput.values.c.toFixed(2).toString(),
      ppu: state.form.EllipsoidInput.values.ppu
    }
  }

//connects redux actions to props
function mapDispatchToProps(dispatch) {
    return bindActionCreators({
        updateGeometry: updateGeometry,
        updatePattern: updatePattern
    }, dispatch);
  }

export default connect(mapStateToProps, mapDispatchToProps)(Scene);