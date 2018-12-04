import React from 'react';
import { connect } from 'react-redux';
import EllipsoidInput from '../containers/ellipsoidInput';
import ProjectionInput from '../containers/projectionInput';

class InputForm extends React.PureComponent {
  render() {
    return (
      <div>
        <EllipsoidInput />
        <ProjectionInput />
      </div>
    );
  }
}

export default connect(null, null)(InputForm);
