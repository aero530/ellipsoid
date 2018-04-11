import React from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import EllipsoidInput from '../containers/ellipsoidInput';
import ProjectionInput from '../containers/projectionInput';

import {projectionChanged, geometryChanged} from '../actions';

class InputForm extends React.Component {
  constructor(props) {
    super(props);
    this.handleGeometryChanged = this.handleGeometryChanged.bind(this);
    this.handleProjectionChanged = this.handleProjectionChanged.bind(this);
}

handleGeometryChanged(data) {
  this.props.geometryChanged(data);
}

handleProjectionChanged(data) {
  this.props.projectionChanged(data);
}

  render() {
    return (
      <div>
      <EllipsoidInput onChange={this.handleGeometryChanged} />
      <ProjectionInput onChange={this.handleProjectionChanged} />
      </div>
    );
  }

}

//connects redux actions to props
function mapDispatchToProps(dispatch) {
  return bindActionCreators({
    geometryChanged: geometryChanged,
    projectionChanged: projectionChanged
  }, dispatch);
}

export default connect(null, mapDispatchToProps)(InputForm);