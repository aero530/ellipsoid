import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';
import ellipsoidReducer from './ellipsoid';
import geometryReducer from './geometryReducer';
import projectionReducer from './projectionReducer';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer,
  geometrySettings: geometryReducer,
  projectionSettings: projectionReducer,
  ellipsoid: ellipsoidReducer
});

export default rootReducer;