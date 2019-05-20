import { combineReducers } from 'redux';
import { connectRouter } from 'connected-react-router';
import input from './input';
import geometry from './geometry';
import edges from './edges';

export default function createRootReducer(history) {
  return combineReducers({
    router: connectRouter(history),
    input,
    geometry,
    edges,
  });
}
