import React, { Component } from 'react';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';
import Dropzone from 'react-dropzone';

import { fileImport } from '../actions'; // FILEIMPORT

class DropZone extends Component {
  handleImportFile(acceptedFiles) {
    const file = acceptedFiles[0];
    const reader = new FileReader();
    reader.onload = function (event) {
      const output = reader.result;
      this.props.fileImport(output);
    }.bind(this);
    reader.readAsText(file);
  }

  render() {
    return (
      <div className="dropzone">
        <Dropzone accept="image/svg+xml" onDrop={this.handleImportFile.bind(this)}>
          <p>Try dropping some files here, or click to select files to upload.</p>
          <p>Only *.svg images will be accepted</p>
        </Dropzone>
      </div>
    );
  }
}

//connects redux actions to props
function mapDispatchToProps(dispatch) {
  return bindActionCreators({
    fileImport: fileImport,
  }, dispatch);
}

export default connect(null, mapDispatchToProps)(DropZone);
