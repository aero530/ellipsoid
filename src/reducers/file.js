import { FILEIMPORT } from '../actions/types';

export default function(state="", action) {
  switch (action.type) {
    case FILEIMPORT:
      return action.payload;
    default:
      return state;
  }
}