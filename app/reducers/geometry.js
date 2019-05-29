import { UPDATE_GEOMETRY } from '../actions';

const initialState = {
  geometry: [],
  indexWide: 0,
  divisions: 0,
  Divisions: 0,
  obj: '',
};

export default function (state = initialState, action) {
  switch (action.type) {
    case UPDATE_GEOMETRY:
      return {
        ...state,
        ...action.value,
      };
    default:
      return state;
  }
}
