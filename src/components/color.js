import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import { PaperContainer, Rectangle, Circle, Path, Layer } from '@psychobolt/react-paperjs';
import * as paper from 'paper';

import {changeColor} from '../actions';


const Shapes = () => (
  <Circle center={[120, 50]} radius={35} fillColor="#00FF00" />
);

const pathData = 'M100,50 c0,20 -18,50 -50,50 S0,70,0,50 S22.386,0,50,0 S100,22.386,100,50';
const canvasWidth = 400;
const canvasHeight = 400;

class Color extends Component {
  
  constructor(props) {
    super(props);
    this.handleChangeColor = this.handleChangeColor.bind(this);
  }
  
  handleChangeColor(e) {
    this.props.changeColor([Math.floor(Math.random() * 256),Math.floor(Math.random() * 256),Math.floor(Math.random() * 256)]);
  }
  
  render() {
    return (
      <div>
        <p>Color:</p>
        <ul>
          {this.props.color.map( (val, idx) => 
              <li key={idx}>{val}</li>
            )}
        </ul>
        <p>
          {' '}
          <button onClick={this.handleChangeColor}>Change Color</button>
          {' '}
        </p>
        <PaperContainer canvasProps={{ width: canvasWidth, height: canvasHeight }}>
        <Circle center={[180, 50]} radius={60} fillColor={new paper.Color(this.props.color[0]/255, this.props.color[1]/255, this.props.color[2]/255)} />
        <Layer>
          <Path pathData={pathData} fillColor="blue" />
          <Rectangle x={15} y={175} width={100} height={100} fillColor="red" />
        </Layer>
        <Layer>
          <Shapes />
        </Layer>
        <Circle center={[220, 50]} radius={35} fillColor="yellow" />
      </PaperContainer>
      
      </div>
    );
  }
}
  
//connects root reducer to props
function mapStateToProps(state) {
  return {
    color: state.color
  }
}
  
//connects redux actions to props
function mapDispatchToProps(dispatch) {
  return bindActionCreators({
    changeColor: changeColor
  }, dispatch);
}
  
export default connect(mapStateToProps, mapDispatchToProps)(Color);