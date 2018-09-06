import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form';
import geometryReducer from './geometry';
import fileReducer from './file';

const rootReducer = combineReducers({
  form: formReducer,
  shape: geometryReducer,
  file: fileReducer
});

export default rootReducer;