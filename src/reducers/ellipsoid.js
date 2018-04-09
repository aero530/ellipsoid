import { UPDATEELLIPSOID } from '../actions/types';

export default function(state="default", action) {
  switch (action.type) {
    case UPDATEELLIPSOID:
      return action.payload;
    default:
      return state;
  }
}