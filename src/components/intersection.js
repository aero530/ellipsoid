// import * as paper from 'paper';

export default function intersection(args) {
    
    let onmessage = e => { // eslint-disable-line no-unused-vars
        // Write your code here...
        // console.log(e);
        
        // let project = new paper.Project();
        // let textureArray = project.importJSON(e.data.texture);
        // let clip = project.importJSON(e.data.clipRegion);

        // let texturePanel = textureArray.intersect(clip);
        // // texturePanel.fillColor = new paper.Color(...e.data.color);
        // texturePanel.name = e.data.name;
        // let output = texturePanel.exportJSON();

        postMessage("output");
    };
}
