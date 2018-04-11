import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';
import geometryReducer from './geometry';
import inputs from './inputs';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer,
  inputs: inputs,
  shape: geometryReducer
});

export default rootReducer;