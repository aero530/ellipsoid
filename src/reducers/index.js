import { combineReducers } from 'redux';
import { reducer as formReducer } from 'redux-form'
import counterReducer from './counter';
import colorReducer from './color';

const rootReducer = combineReducers({
  counter: counterReducer,
  color: colorReducer,
  form: formReducer
});

export default rootReducer;