import { UPDATEGEOMETRY } from '../actions/types';

export default function(state="default", action) {
  switch (action.type) {
    case UPDATEGEOMETRY:
      return action.payload;
    default:
      return state;
  }
}