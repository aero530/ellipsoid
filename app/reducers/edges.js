import { UPDATE_EDGES } from '../actions';

const initialState = {
  edgesFlat: [],
  edges: [],
  indexWide: 0,
  obj: '',
};

export default function (state = initialState, action) {
  switch (action.type) {
    case UPDATE_EDGES:
      return {
        ...state,
        ...action.value,
      };
    default:
      return state;
  }
}
