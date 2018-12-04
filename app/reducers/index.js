import { combineReducers } from 'redux';
import { connectRouter } from 'connected-react-router';
import input from './input';
import geometry from './geometry';
import panels from './panels';

export default function createRootReducer(history) {
  return combineReducers({
    router: connectRouter(history),
    input,
    geometry,
    panels,
  });
}
