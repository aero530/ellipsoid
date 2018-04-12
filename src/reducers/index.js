import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';
import geometryReducer from './geometry';
import fileReducer from './file';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer,
  shape: geometryReducer,
  file: fileReducer
});

export default rootReducer;