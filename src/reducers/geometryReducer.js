import { GEOMETRYCHANGED } from '../actions/types';

export default function(state="default", action) {
  switch (action.type) {
    case GEOMETRYCHANGED:
      return action.payload;
    default:
      return state;
  }
}