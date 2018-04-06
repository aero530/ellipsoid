import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import * as paper from 'paper'

class Scene extends Component {
    constructor(props) {
        super(props)
        window.paper = new paper.PaperScope();
        this.handleDownload = this.handleDownload.bind(this);
    }

    handleDownload(e) {
        const scope = window.paper;
        const url = "data:image/svg+xml;utf8," + encodeURIComponent(scope.project.exportSVG({asString:true}));
        const link = document.createElement("a");
        link.download = "image.svg";
        link.href = url;
        link.click();
    }
    
    render() {
        return (
            <div>
            <canvas id="paper" width={500} height={500} />
            <button onClick={this.handleDownload}>Download</button>
            </div>
        )
    }
    componentDidUpdate() {
        let scope = window.paper;
        scope.setup(document.getElementById('paper'));

        console.log(scope);
        var path = new scope.Path();
		// Give the stroke a color
		path.strokeColor = new scope.Color(this.props.color[0]/255, this.props.color[1]/255, this.props.color[2]/255);
		var start = new scope.Point(100, 100);
		// Move to start and draw a line from there
		path.moveTo(start);
		// Note that the plus operator on Point objects does not work
		// in JavaScript. Instead, we need to call the add() function:
        path.lineTo(start.add([ 200, -50 ]));
        
        var firstLayer = scope.project.activeLayer;
        firstLayer.name = 'Layer 1';

        var secondLayer = new scope.Layer();
        secondLayer.name = 'Layer 2';

        firstLayer.activate();

        var circle = new scope.Shape.Circle(new scope.Point(50,75), 30);
        circle.strokeColor = "#333333";
        circle.fillColor = "#0084B0";

        secondLayer.activate();
        var ellipse = new scope.Shape.Ellipse(new scope.Point(20,100), new scope.Size(100, 20));
        ellipse.fillColor = new scope.Color(this.props.color[0]/255, this.props.color[1]/255, this.props.color[2]/255);


    }
}



//connects root reducer to props
function mapStateToProps(state) {
    return {
      color: state.color
    }
  }
    
  //connects redux actions to props
//   function mapDispatchToProps(dispatch) {
//     return bindActionCreators({
//       changeColor: changeColor
//     }, dispatch);
//   }
    
//   export default connect(mapStateToProps, mapDispatchToProps)(Scene);
export default connect(mapStateToProps, null)(Scene);