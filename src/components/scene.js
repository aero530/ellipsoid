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
        this.getUnits = this.getUnits.bind(this);
        this.handleTexture = this.handleTexture.bind(this);
        this.handleDownload = this.handleDownload.bind(this);
        this.handleUpdateGeometry = this.handleUpdateGeometry.bind(this);
        this.handleUpdatePattern = this.handleUpdatePattern.bind(this);
    }

    getUnits(ppu) {
        if (ppu === 96 || ppu === "96") {
            return "in";
        } else if (ppu === 3.7795276 || ppu === "3.7795276") {
            return "mm";
        } else if (ppu === 37.795276 || ppu === "37.795276") {
            return "cm";
        }
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

        let units = this.getUnits(this.props.ppu);
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

        scope.project.layers['Pattern Source Quadrilaterals'].visible = false;
        scope.project.layers['Pattern Destination Quadrilaterals'].visible = false;

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
            var notesLayer = new scope.Layer();
            notesLayer.name = 'Notes';
            notesLayer.activate();

            console.log(this.props.ppu);

            let units = this.getUnits(this.props.ppu);

            const filename = "ellipsoid_a" + this.props.a + units +
                "_b" + this.props.b + units +
                "_c" + this.props.c + units +
                ".svg";

            let textFilename = new scope.PointText({
                    point: [0, 0],
                    content: filename,
                    fillColor: 'black',
                    fontFamily: 'Roboto',
                    fontSize: 0.25*this.props.ppu
                });
                
                textFilename.rotate(-90, textFilename.bounds.bottomRight);
                textFilename.position.x = 0.15*this.props.ppu;
                textFilename.position.y = scope.project.layers['Bounding Box'].bounds.height - textFilename.bounds.height/2 - 0.15*this.props.ppu;

            new scope.PointText({
                point: [0.1*this.props.ppu, .15*this.props.ppu],
                content: JSON.stringify(this.props.geometrySettings),
                fillColor: 'black',
                fontFamily: 'Courier New',
                fontSize: 0.2*this.props.ppu
            });

            // Draw a ruler on the bottom of the pattern based on the units specified

            var path = new scope.Path();
            // Give the stroke a color
            path.strokeColor = new scope.Color(.7,.3,.5);
            path.strokeWidth = 0.01*this.props.ppu;
            // var start = new scope.Point(0.1*this.props.ppu, scope.project.layers['Bounding Box'].bounds.height);
            var start = new scope.Point(scope.project.layers['Ellipsoid Pattern'].bounds.x, scope.project.layers['Bounding Box'].bounds.height);
            // Move to start and draw a line from there
            path.moveTo(start);
            // Note that the plus operator on Point objects does not work
            // in JavaScript. Instead, we need to call the add() function:
            path.lineTo(start.add([ 0, -0.3*this.props.ppu ]));

            for (var i = 0; i < scope.project.layers['Ellipsoid Pattern'].bounds.width / this.props.ppu; i++) {
                var copy = path.clone();
                // Distribute the copies horizontally, so we can see them:
                copy.position.x += i * this.props.geometrySettings.ppu;
                new scope.PointText({
                    point: copy.position,
                    content: i+units,
                    fillColor: 'black',
                    fontFamily: 'Roboto',
                    fontSize: 0.2*this.props.ppu
                });
            }
        }
    }

    handleTexture() {
        const textureSVG = this.props.texture;
        const scale = .5

        const scope = window.paper;

        if (textureSVG !== "") {
            const patternLayer = scope.project.layers['Ellipsoid Pattern'];

            let patternWidth = scope.project.layers['Bounding Box'].bounds.width;
            let patternHeight = scope.project.layers['Bounding Box'].bounds.height;

            const textureSourceLayer = new scope.Layer();
            textureSourceLayer.name = 'Texture Source';
            textureSourceLayer.activate();

            // get all the items from the import that are paths
  
            let inputPaths = scope.project.importSVG(textureSVG, {insert:false}).getItems({class: "Path"});

            // create new compound path to hold the imported paths
            let texture = new scope.CompoundPath();

            // copy all the imported paths to the compound path and set its color, name, and scale
            texture.addChildren(inputPaths);
            
            texture.fillColor = new scope.Color(0, .5, .2);
            texture.scale(scale,new scope.Point(0,0));
            texture.position = new scope.Point(scope.project.layers['Bounding Box'].bounds.center.x - texture.bounds.width/2, scope.project.layers['Bounding Box'].bounds.center.y - texture.bounds.height/2);
            console.log(texture);

            let countX = patternWidth/texture.bounds.width;
            let countY = patternHeight/texture.bounds.height;
            
            // create a new compound path for the arrayed texture
            let textureArray = new scope.CompoundPath();

            // array (clone) the texture path to cover the entire pattern
            for (let i = -Math.ceil(countX/2); i < Math.ceil(countX/2)+1; i++) {
                for (let j = -Math.ceil(countY/2); j< Math.ceil(countY/2)+1; j++) {
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
            console.log(textureArray);
   
            // don't need the source texture input anymore
            texture.remove();

            // Intersect the textureArray compound path with each source qualrilateral
            let panelCount = scope.project.layers['Pattern Source Quadrilaterals'].children.length;
            
            for (let i = 0; i < panelCount; i++) {
                let texturePanel = textureArray.intersect(scope.project.layers['Pattern Source Quadrilaterals'].children[i]);
                texturePanel.fillColor = new scope.Color(.5, .5, .5, .2);
                texturePanel.name = 'texture-'+i;
                console.log(texturePanel);
            }

            // don't need the source textureArray anymore
            textureArray.remove();
    
            const textureDestLayer = new scope.Layer();
            textureDestLayer.name = 'Texture Mapped';
            textureDestLayer.activate();

            for (let indexp = 0; indexp < panelCount; indexp++) {

                const srcCorners = [];
                const dstCorners = [];

                for (let j=0; j<4; j++) {
                    srcCorners.push(scope.project.layers['Pattern Source Quadrilaterals'].children[indexp].segments[j].point.x);
                    srcCorners.push(scope.project.layers['Pattern Source Quadrilaterals'].children[indexp].segments[j].point.y);
                    dstCorners.push(scope.project.layers['Pattern Destination Quadrilaterals'].children[indexp].segments[j].point.x);
                    dstCorners.push(scope.project.layers['Pattern Destination Quadrilaterals'].children[indexp].segments[j].point.y);
                }
                
                const perspT = perspective(srcCorners, dstCorners);

                let paths = [];

                if (textureSourceLayer.children['texture-'+indexp].className === "CompoundPath") {
                    // console.log("compound path");
                    const pathList = textureSourceLayer.children['texture-'+indexp].getItems({class: "Path"});
                    pathList.forEach(function(element) {
                        paths.push(element.clone());
                    });
                } else if (textureSourceLayer.children['texture-'+indexp].className  === "Path") {
                    // console.log("path");
                    paths = [ textureSourceLayer.children['texture-'+indexp].clone() ];
                } else {
                    console.error("Error - Not a valid path type for texture");
                    console.error(textureSourceLayer.children['texture-'+indexp]);
                }
                
                let textureMapped = new scope.CompoundPath();

                for (let p=0; p<paths.length; p++) {
                    for (let j=0; j<paths[p].segments.length; j++) {
                        const tempX = paths[p].segments[j].point.x;
                        const tempY = paths[p].segments[j].point.y;
                        const dstPt = perspT.transform(tempX, tempY);
                        paths[p].segments[j].point.x = dstPt[0];
                        paths[p].segments[j].point.y = dstPt[1];
                    }
                }
                textureMapped.addChildren(paths);
                textureMapped.fillColor = new scope.Color(.5, .5, 1, 0);
                console.log(textureMapped);    
                paths = [];

            }
            
            textureSourceLayer.remove();

            // reactivate the pattern layer for good measure
            patternLayer.activate();

            console.log(scope);
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
        //this.handleTexture();
    }

    componentDidMount() {
        const geometry = computeGeometry(this.props.geometrySettings);
        this.handleUpdateGeometry(geometry);

        const pattern = computePattern(geometry,  this.props.geometrySettings, this.props.projectionSettings.projection);
        this.handleUpdatePattern(pattern);

        this.handleDrawPattern(pattern);
        //this.handleTexture();
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
      imageOffset: state.form.ProjectionInput.values.imageOffset.toFixed(2),
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