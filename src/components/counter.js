import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';

import {incrementCounter, decrementCounter} from '../actions';

class Counter extends Component {
  constructor(props) {
    super(props);
    this.handleIncrementCounter = this.handleIncrementCounter.bind(this);
    this.handleDecrementCounter = this.handleDecrementCounter.bind(this);
  }
 
  handleIncrementCounter(e) {
    this.props.incrementCounter(1);
  }
  handleDecrementCounter(e) {
    this.props.decrementCounter(1);
  }
  
  render() {
    return (
      <p>Clicked: {this.props.value} times
        {' '}
        <button onClick={this.handleIncrementCounter}>+</button>
        {' '}
        <button onClick={this.handleDecrementCounter}>-</button>
        {' '}
      </p>
    );
  }
}
  
//connects root reducer to props
function mapStateToProps(state) {
  return {
    value: state.counter
  }
}
  
//connects redux actions to props
function mapDispatchToProps(dispatch) {
  return bindActionCreators({
    incrementCounter: incrementCounter, 
    decrementCounter: decrementCounter
  }, dispatch);
}

export default connect(mapStateToProps, mapDispatchToProps)(Counter);