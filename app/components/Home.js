import React, { Component } from 'react';
import compose from 'recompose/compose';
import { connect } from 'react-redux';
import { withStyles } from '@material-ui/core/styles';
import Grid from '@material-ui/core/Grid';
import Paper from '@material-ui/core/Paper';

import ReactResizeDetector  from 'react-resize-detector';
import Typography from '@material-ui/core/Typography';

import EllipsoidInput from './ellipsoidInput';
import ProjectionInput from './projectionInput';
import Scene from './scene';
import View3D from './view3D';


const styles = theme => ({
  root: {
    // width: '100%',
    padding: theme.spacing.unit * 2,
  },
  paper: {
    backgroundColor: theme.palette.background.paper,
    boxShadow: theme.shadows[5],
    padding: theme.spacing.unit * 2,
    margin: theme.spacing.unit * 2,
  },
  title: {
    paddingTop: theme.spacing.unit * 2,
  }
});

class Home extends Component {
  render() {
    const { classes } = this.props;
    return (
      <div className={classes.root} >
        <Grid container className={classes.root} spacing={16}>
          
          <Grid item xs={12} md={12} lg={4}>
            <Paper className={classes.paper} >
              <Grid container>

                <Grid item xs={6} md={6} lg={12}>
                  <Typography className={classes.title} variant="h6" color="inherit">
                    Geometry Settings
                  </Typography>
                  <EllipsoidInput />
                </Grid>

                <Grid item xs={6} md={6} lg={12}>
                  <Typography className={classes.title} variant="h6" color="inherit">
                    Projection Settings
                  </Typography>
                  <ProjectionInput />
                </Grid>

              </Grid>
              
            </Paper>
          </Grid>
          
          <Grid item xs={12} md={6} lg={4}>
            <Paper className={classes.paper} >
              <Typography className={classes.title} variant="h6" color="inherit">
                3D Geometry
              </Typography>
              <ReactResizeDetector handleWidth handleHeight onResize={this.onResize}>
                {(width) => {
                  return(<View3D id="edges" size={`${width}px`} />);
                }}
              </ReactResizeDetector>
            </Paper>
          </Grid>

          <Grid item xs={12} md={6} lg={4}>
            <Paper className={classes.paper} >
              <Typography className={classes.title} variant="h6" color="inherit">
                Flattened Pattern
              </Typography>
              <ReactResizeDetector handleWidth handleHeight onResize={this.onResize}>
                {(width) => {
                  return(<View3D id="edgesFlat" size={`${width}px`} />);
                }}
              </ReactResizeDetector>
            </Paper>
          </Grid>

          <Grid item xs={12}>
            <Paper id="scene" className={classes.paper} >
              <Typography className={classes.title} variant="h6" color="inherit">
                SVG Pattern
              </Typography>
              <Scene />
            </Paper>
          </Grid>
        
        </Grid>
      </div>
    );
  }
}


const mapStateToProps = () => ({
});

const mapDispatchToProps = () => ({
});
  
export default compose(
  withStyles(styles),
  connect(
    mapStateToProps,
    mapDispatchToProps,
  ),
)(Home);
