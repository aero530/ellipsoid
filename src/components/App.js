import React, { Component } from 'react';
import {connect} from 'react-redux';

import Counter from './counter'
import Color from './color'
import Scene from './scene'
import InputForm from './inputForm'

// Use paper.js on the back end to create the objects then render to svg and spit into an svg bucket (ie dont use canvas at all)

class App extends Component {
  
  render() {
    return (
      <div className="App container">
      <InputForm />
      <Counter />
      <Color />
      <Scene />
      </div>
    );
  }
}

export default connect(null, null)(App);