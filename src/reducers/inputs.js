import { PROJECTIONCHANGED } from '../actions/types';
import { GEOMETRYCHANGED } from '../actions/types';

export default function(state={}, action) {
  switch (action.type) {
    case GEOMETRYCHANGED:
      return {
        ...state,         
        ...action.payload
      };
    case PROJECTIONCHANGED:
      return {
        ...state,         
        ...action.payload
      };
    default:
      return state;
  }
}