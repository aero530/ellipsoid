
import * as perspective from 'perspective-transform';

export function processTexture(textureSVG, scale, scope) {
    // const textureSVG = this.props.texture;
    // const scale = .5
    // const scope = window.paper;

    if (textureSVG !== "") {
        const patternLayer = scope.project.layers['Ellipsoid Pattern'];

        let patternWidth = scope.project.layers['Bounding Box'].bounds.width;
        let patternHeight = scope.project.layers['Bounding Box'].bounds.height;

        const textureSourceLayer = new scope.Layer();
        textureSourceLayer.name = 'Texture Source';
        textureSourceLayer.activate();

        // get all the items from the import that are paths

        let inputPaths = scope.project.importSVG(textureSVG, {insert:false}).getItems({class: "Path"});

        // create new compound path to hold the imported paths
        let texture = new scope.CompoundPath();

        // copy all the imported paths to the compound path and set its color, name, and scale
        texture.addChildren(inputPaths);
        
        texture.fillColor = new scope.Color(0, .5, .2);
        texture.scale(scale,new scope.Point(0,0));
        texture.position = new scope.Point(scope.project.layers['Bounding Box'].bounds.center.x - texture.bounds.width/2, scope.project.layers['Bounding Box'].bounds.center.y - texture.bounds.height/2);

        let countX = patternWidth/texture.bounds.width;
        let countY = patternHeight/texture.bounds.height;
        
        // create a new compound path for the arrayed texture
        let textureArray = new scope.CompoundPath();

        // array (clone) the texture path to cover the entire pattern
        for (let i = -Math.ceil(countX/2); i < Math.ceil(countX/2)+1; i++) {
            for (let j = -Math.ceil(countY/2); j< Math.ceil(countY/2)+1; j++) {
                let copy = texture.clone();
                // Shift copy to new location
                copy.position.x += i * copy.bounds.width;
                copy.position.y += j * copy.bounds.height;
                // put copy's content into textureArray then remove it
                textureArray.addChildren(copy.getItems({class: "Path"}));
                copy.remove();
            }  
        }
        textureArray.fillColor = new scope.Color(1, 0, 0);
        // console.log(textureArray);

        // don't need the source texture input anymore
        texture.remove();

        // Intersect the textureArray compound path with each source qualrilateral
        let panelCount = scope.project.layers['Pattern Source Quadrilaterals'].children.length;
        
        for (let i = 0; i < panelCount; i++) {
            let texturePanel = textureArray.intersect(scope.project.layers['Pattern Source Quadrilaterals'].children[i]);
            texturePanel.fillColor = new scope.Color(.5, .5, .5, .2);
            texturePanel.name = 'texture-'+i;
            // console.log(texturePanel);
        }

        // don't need the source textureArray anymore
        textureArray.remove();

        const textureDestLayer = new scope.Layer();
        textureDestLayer.name = 'Texture Mapped';
        textureDestLayer.activate();

        for (let indexp = 0; indexp < panelCount; indexp++) {

            const srcCorners = [];
            const dstCorners = [];

            for (let j=0; j<4; j++) {
                srcCorners.push(scope.project.layers['Pattern Source Quadrilaterals'].children[indexp].segments[j].point.x);
                srcCorners.push(scope.project.layers['Pattern Source Quadrilaterals'].children[indexp].segments[j].point.y);
                dstCorners.push(scope.project.layers['Pattern Destination Quadrilaterals'].children[indexp].segments[j].point.x);
                dstCorners.push(scope.project.layers['Pattern Destination Quadrilaterals'].children[indexp].segments[j].point.y);
            }
            
            const perspT = perspective(srcCorners, dstCorners);

            let paths = [];

            if (textureSourceLayer.children['texture-'+indexp].className === "CompoundPath") {
                // console.log("compound path");
                const pathList = textureSourceLayer.children['texture-'+indexp].getItems({class: "Path"});
                pathList.forEach(function(element) {
                    paths.push(element.clone());
                });
            } else if (textureSourceLayer.children['texture-'+indexp].className  === "Path") {
                // console.log("path");
                paths = [ textureSourceLayer.children['texture-'+indexp].clone() ];
            } else {
                console.error("Error - Not a valid path type for texture");
                console.error(textureSourceLayer.children['texture-'+indexp]);
            }
            
            let textureMapped = new scope.CompoundPath();

            for (let p=0; p<paths.length; p++) {
                for (let j=0; j<paths[p].segments.length; j++) {
                    const tempX = paths[p].segments[j].point.x;
                    const tempY = paths[p].segments[j].point.y;
                    const dstPt = perspT.transform(tempX, tempY);
                    paths[p].segments[j].point.x = dstPt[0];
                    paths[p].segments[j].point.y = dstPt[1];
                }
            }
            textureMapped.addChildren(paths);
            textureMapped.fillColor = new scope.Color(.5, .5, 1, .5);
            // console.log(textureMapped);    
            paths = [];
        }
        
        textureSourceLayer.remove();

        // reactivate the pattern layer for good measure
        patternLayer.activate();
    }
}