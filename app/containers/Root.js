import React, { Component } from 'react';
import compose from 'recompose/compose';
import { Provider } from 'react-redux';
import { ConnectedRouter } from 'connected-react-router';
import CssBaseline from '@material-ui/core/CssBaseline';
import {
  createMuiTheme,
  withStyles,
  MuiThemeProvider,
} from '@material-ui/core/styles';

import Routes from '../Routes';


/** @constant
 * Default theme settings
 * https://material-ui.com/customization/default-theme/
*/
const theme = createMuiTheme({
  palette: {
    contrastThreshold: 3,
    tonalOffset: 0.2,
    background: {
      paper: '#fafafa',
      default: '#eee',
    },
    primary: {
      light: '#ea96b9',
      main: '#9f1d54',
      dark: '#72173d',
      contrastText: '#ffffff',
    },
  },
  overrides: {
  },
});

const muiStyles = () => ({
  root: {},
});

class Root extends Component {
// export default class Root extends Component {
  render() {
    const { store, history } = this.props;
    return (
      <MuiThemeProvider theme={theme}>
        <CssBaseline />
        <Provider store={store}>
          <ConnectedRouter history={history}>
            <Routes />
          </ConnectedRouter>
        </Provider>
      </MuiThemeProvider>
    );
  }
}

export default compose(withStyles(muiStyles))(Root);
