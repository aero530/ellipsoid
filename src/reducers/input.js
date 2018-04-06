import { CHANGEINPUT } from '../actions/types';

export default function(state={}, action) {
  switch (action.type) {
    case CHANGEINPUT:
      //return state + action.payload;
      return state;
    default:
      return state;
  }
}