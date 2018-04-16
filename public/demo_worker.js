//importScripts('https://cdnjs.cloudflare.com/ajax/libs/paper.js/0.11.5/paper-core.js');

importScripts('paper-core.js');

onmessage = e => { // eslint-disable-line no-unused-vars
    //console.log(e);
    let project = new paper.Project();
    let textureArray = project.importJSON(e.data.texture);
    let clip = project.importJSON(e.data.clipRegion);
    
    let texturePanel = textureArray.intersect(clip);
    texturePanel.fillColor = new paper.Color(e.data.color[0],e.data.color[1],e.data.color[2],e.data.color[3]);
    // texturePanel.fillColor = new paper.Color(.5, .5, .5, .2)
    texturePanel.name = "texture-"+e.data.name;

    let output = texturePanel.exportJSON();
    project.clear();
    postMessage(output);
};

