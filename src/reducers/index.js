import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';
import geometryReducer from './geometry';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer,
  shape: geometryReducer
});

export default rootReducer;