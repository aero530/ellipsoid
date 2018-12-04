import {
  UPDATEGEOMETRY,
  UPDATEPATTERN,
} from '../actions/types';

export default function (state = {}, action) {
  switch (action.type) {
    case UPDATEGEOMETRY:
      return {
        ...state,
        ...action.payload,
      };
    case UPDATEPATTERN:
      return {
        ...state,
        ...action.payload,
      };
    default:
      return state;
  }
}
