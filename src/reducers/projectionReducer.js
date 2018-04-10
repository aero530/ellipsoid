import { PROJECTIONCHANGED } from '../actions/types';

export default function(state="default", action) {
  switch (action.type) {
    case PROJECTIONCHANGED:
      return action.payload;
    default:
      return state;
  }
}