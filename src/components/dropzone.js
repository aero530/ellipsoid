import React, { Component } from 'react';
import {connect} from 'react-redux';
import Dropzone from 'react-dropzone';

class DropZone extends Component {

    handleImportFile(acceptedFiles) {
        var file = acceptedFiles[0];
        var reader  = new FileReader();
        reader.onload = function(event) {
            let output = reader.result;
            document.getElementById("demo").innerHTML = output;
        }
        reader.readAsText(file);
    }
    render() {
      return (
            <div className="dropzone">
                <Dropzone accept="image/svg+xml" onDrop={this.handleImportFile.bind(this)}>
                <p>Try dropping some files here, or click to select files to upload.</p>
                <p>Only *.svg images will be accepted</p>
                </Dropzone>
                <div id="demo" />
            </div>
        );
    }
}
export default connect(null, null)(DropZone);