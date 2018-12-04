import { UPDATE_PANELS } from '../actions';

const initialState = {
  panelsFlat: [],
  panels: [],
  indexWide: 0,
};

export default function (state = initialState, action) {
  switch (action.type) {
    case UPDATE_PANELS:
      return {
        ...state,
        ...action.value,
      };
    default:
      return state;
  }
}
