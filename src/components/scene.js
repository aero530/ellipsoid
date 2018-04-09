import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import * as paper from 'paper';
import {drawPattern, computePattern} from './ellipsoid.js';

import {updateEllipsoid} from '../actions';

class Scene extends Component {
    constructor(props) {
        super(props)
        window.paper = new paper.PaperScope();
        this.handleDownload = this.handleDownload.bind(this);
        this.handleUpdateEllipsoid = this.handleUpdateEllipsoid.bind(this);
    }

    handleUpdateEllipsoid(data) {
        this.props.updateEllipsoid(data);
      }

    handleDownload() {
        const scope = window.paper;
        // export bounds : content to export full size image (not scaled down view)
        const url = "data:image/svg+xml;utf8," + encodeURIComponent(scope.project.exportSVG({bounds:"content", asString:true}));
        
        //replace %3Cg with %3Cg%20inkscape%3Agroupmode%20%3D%20%22layer%22

        // THIS CURRENTLY ISNT WORKING
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        url.replace("<g", '<g inkscape:groupmode="layer"');
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //
        //

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
    
    render() {
        return (
            <div>
            <canvas id="paper" width={600} height={600} />
            <button onClick={this.handleDownload}>Download</button>
            </div>
        )
    }
    componentDidUpdate() {
        let scope = window.paper;

        if (scope.projects.length === 0) {  // if there is not a project define (first time this is loaded)
            scope.setup(document.getElementById('paper')); // setup the project
        } else {
            scope.project.clear(); // clear the project of all content
        }

        var patternLayer = scope.project.activeLayer;
        patternLayer.name = 'Ellipsoid Pattern';

        const ellipsoid = computePattern(this.props.inputstate);

        drawPattern(this.props.inputstate, ellipsoid, scope);
        this.handleUpdateEllipsoid(ellipsoid);
        
        let imgWidth = scope.project.activeLayer.bounds.width
        let imgHeight = scope.project.activeLayer.bounds.height;

        let viewWidth = scope.view.viewSize.width
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
}

//connects root reducer to props
function mapStateToProps(state) {
    return {
      //color: state.color,
      inputstate: state.form.EllipsoidInput.values,
      a: state.form.EllipsoidInput.values.a.toFixed(2).toString(),
      b: state.form.EllipsoidInput.values.b.toFixed(2).toString(),
      c: state.form.EllipsoidInput.values.c.toFixed(2).toString(),
      ppu: state.form.EllipsoidInput.values.ppu
    }
  }




//connects redux actions to props
function mapDispatchToProps(dispatch) {
    return bindActionCreators({
        updateEllipsoid: updateEllipsoid
    }, dispatch);
  }

//   export default connect(mapStateToProps, mapDispatchToProps)(Scene);
//export default connect(mapStateToProps, null)(Scene);
export default connect(mapStateToProps, mapDispatchToProps)(Scene);