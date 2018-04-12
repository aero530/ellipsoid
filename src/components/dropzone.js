import React, { Component } from 'react';
import {connect} from 'react-redux';
import {bindActionCreators} from 'redux';
import Dropzone from 'react-dropzone';

import {fileImport} from '../actions'; //FILEIMPORT

class DropZone extends Component {
      
    handleImportFile(acceptedFiles) {
        let file = acceptedFiles[0];
        let reader  = new FileReader();
        reader.onload = function(event) {
            let output = reader.result;
            document.getElementById("demo").innerHTML = output;

            // // Draw svg to canvas....may remove this after getting paper working with this data
            // change to <canvas id="demo" />
            // let canvas = document.getElementById('demo');
            // let ctx = canvas.getContext('2d');
            // let data = encodeURIComponent(output);
            // let img = new Image();
            // img.onload = function() {
            //   ctx.drawImage(img, 0, 0);
            //   canvas.toBlob(function(blob) {
            //      var newImg = document.createElement('img'),
            //      url = URL.createObjectURL(blob);
            //      newImg.onload = function() {
            //      // no longer need to read the blob so it's revoked
            //      URL.revokeObjectURL(url);
            //    };
            //    newImg.src = url;
            //    document.body.appendChild(newImg);
            //  });
            // }
            // img.src = "data:image/svg+xml," + data;
            
            // Send the svg data to the fileImport action
            this.props.fileImport(output);
        }.bind(this)
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
    
//connects redux actions to props
function mapDispatchToProps(dispatch) {
    return bindActionCreators({
        fileImport: fileImport
    }, dispatch);
}
    

export default connect(null, mapDispatchToProps)(DropZone);