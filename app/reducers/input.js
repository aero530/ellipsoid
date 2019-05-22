import { UPDATE_INPUT } from '../actions';

const initialState = {
  a: 3.75, // in - major axis radius
  b: 2.875, // in - minor axis radius
  c: 3, // in - height axis radius
  hTop: 0, // in - height added to bottom of ellipsoid
  hMiddle: 2,
  hBottom: 2,
  hTopFraction: 1.0,
  hTopShift: 0,
  Divisions: 8, // divisions around major / minor direction
  divisions: 16, // divisions in height
  ppu: 96, // pixels per unit (in)  This is the standard ppi for Inkscape
  thetaMin: -35,
  thetaMax: 90,
  imageOffset: 0.5, // in
  minGap: 0.001, // in
  projection: 'cylindrical', // circular or cylindrical
  inkscapeLayers: true, // include inkscape style layers in output svg file
};

export default function (state = initialState, action) {
  switch (action.type) {
    case UPDATE_INPUT: 
      return {
        ...state,
        [action.name]: action.value,
      };
    default:
      return state;
  }
}
