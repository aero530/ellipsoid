import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';
import ellipsoidReducer from './ellipsoid';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer,
  ellipsoid: ellipsoidReducer
});

export default rootReducer;