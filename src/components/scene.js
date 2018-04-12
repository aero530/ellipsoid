import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import * as paper from 'paper';
import * as perspective from 'perspective-transform';
import {computeGeometry, computePattern, drawPattern} from './ellipsoid.js';

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

    handleTexture() {
        const textureSVG = this.props.texture;
        const scale = 10;

        const scope = window.paper;

        if (textureSVG !== "") {
            const patternLayer = scope.project.layers['Ellipsoid Pattern'];

            let patternWidth = scope.project.layers['Bounding Box'].bounds.width;
            let patternHeight = scope.project.layers['Bounding Box'].bounds.height;

            const textureLayer = new scope.Layer();
            textureLayer.name = 'Texture';
            textureLayer.activate();

            // get all the items from the import that are paths
            let inputPaths = scope.project.importSVG(textureSVG, {insert:false}).getItems({class: "Path"});

            // create new compound path to hold the imported paths
            let texture = new scope.CompoundPath();

            // copy all the imported paths to the compound path and set its color, name, and scale
            texture.addChildren(inputPaths);
            texture.fillColor = new scope.Color(0, .5, .2);
            texture.scale(scale,new scope.Point(0,0));
            texture.name="source-texture";

            console.log(texture);

            let countX = patternWidth/texture.bounds.width;
            let countY = patternHeight/texture.bounds.height;
            
            // create a new compound path for the arrayed texture
            let textureArray = new scope.CompoundPath();

            // array (clone) the texture path to cover the entire pattern
            for (let i = 0; i < countX; i++) {
                for (let j = 0; j< countY; j++) {
                    let copy = texture.clone();
                    // Shift copy to new location
                    copy.position.x += i * copy.bounds.width;
                    copy.position.y += j * copy.bounds.height;
                    // put copy's content into textureArray then remove it
                    textureArray.addChildren(copy.getItems({class: "Path"}));
                    copy.remove();
                }  
            }
            textureArray.fillColor = new scope.Color(1, 0, 0);

            // don't need the source texture input anymore
            texture.remove();

            console.log(scope);

            
            // Intersect the textureArray compound path with each source qualrilateral
            let panelCount = scope.project.layers['Pattern Source Quadrilaterals'].children.length;

            for (let i = 0; i < panelCount; i++) {
                console.time('Intersect '+i+' of '+panelCount);
                let path1 = scope.project.layers['Pattern Source Quadrilaterals'].children[i].clone();
                let result = textureArray.intersect(path1);
                result.fillColor = new scope.Color(.5, .5, .5);
                result.name = 'pattern-'+i;
                console.timeEnd('Intersect '+i+' of '+panelCount);
                path1.remove();
            }
            
            // don't need the source textureArray anymore
            textureArray.remove();



            for (let i = 0; i < 40; i++) {
                //console.time('Intersect '+i+' of '+panelCount);
                const pathSource = scope.project.layers['Pattern Source Quadrilaterals'].children[i].clone();
                const pathDest = scope.project.layers['Pattern Destination Quadrilaterals'].children[i].clone();
                //console.log(pathSource);
                //console.log(pathDest);

                const srcCorners = [];
                const dstCorners = [];

                for (let j=0; j<4; j++) {
                    srcCorners.push(pathSource.segments[j].point.x);
                    srcCorners.push(pathSource.segments[j].point.y);
                    dstCorners.push(pathDest.segments[j].point.x);
                    dstCorners.push(pathDest.segments[j].point.y);
                }

                const perspT = perspective(srcCorners, dstCorners);

                let stretched = scope.project.layers['Pattern Source Quadrilaterals'].children[i].clone();//scope.project.activeLayer.children['pattern-'+i].clone();
                stretched.fillColor = new scope.Color(.5, .5, 1);

                for (let j=0; j<stretched.segments.length; j++) {
                    const tempX = stretched.segments[j].point.x;
                    const tempY = stretched.segments[j].point.y;
                    const dstPt = perspT.transform(tempX, tempY);
                    stretched.segments[j].point.x = dstPt[0];
                    stretched.segments[j].point.y = dstPt[1];
                }
                //console.log(scope.project.activeLayer.children['pattern-'+i]);

                //console.timeEnd('Intersect '+i+' of '+panelCount);
                
            }

            // reactivate the pattern layer for good measure
            patternLayer.activate();

            console.log(scope);
        }
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
function mapStateToProps(state) {
    return {
      //color: state.color,
      geometrySettings: state.form.EllipsoidInput.values,
      projectionSettings: state.form.ProjectionInput.values,
      a: state.form.EllipsoidInput.values.a.toFixed(2).toString(),
      b: state.form.EllipsoidInput.values.b.toFixed(2).toString(),
      c: state.form.EllipsoidInput.values.c.toFixed(2).toString(),
      ppu: state.form.EllipsoidInput.values.ppu,
      texture: state.file
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