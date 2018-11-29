
import { createStore, compose } from 'redux';
// import { applyMiddleware } from 'redux'
// import thunk from 'redux-thunk';
import reducer from './reducers';

const initialState = {};
// const middleware = [thunk];

const store = createStore(
  reducer, /* preloadedState, */
  initialState,
  compose(
    // applyMiddleware(...middleware),
    window.__REDUX_DEVTOOLS_EXTENSION__ && window.__REDUX_DEVTOOLS_EXTENSION__(),
  ),
);

export default store;
