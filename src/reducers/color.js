import { CHANGECOLOR } from '../actions/types';

export default function(state=[255,0,255], action) {
  switch (action.type) {
    case CHANGECOLOR:
      return action.payload;
    default:
      return state;
  }
}