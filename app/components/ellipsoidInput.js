import React from 'react';
import compose from 'recompose/compose';
import { connect } from 'react-redux';
import TextField from '@material-ui/core/TextField';
import Tooltip from '@material-ui/core/Tooltip';
import { withStyles } from '@material-ui/core/styles';
import Select from '@material-ui/core/Select';
import MenuItem from '@material-ui/core/MenuItem';

import Input from '@material-ui/core/Input';

import { UPDATE_INPUT } from '../actions';

const styles = theme => ({
  root: {
    width: '100%',
    marginTop: theme.spacing.unit * 3,
  },
  paper: {
    width: '100%',
    backgroundColor: theme.palette.background.paper,
    boxShadow: theme.shadows[5],
    padding: theme.spacing.unit * 4,
    marginBottom: theme.spacing.unit * 4,
  },
  button: {
    margin: theme.spacing.unit,
  },
  textInput: {
    padding: theme.spacing.unit,
    width: theme.spacing.unit * 15,
  }
});

// const lessThan = otherField =>
//   (value, previousValue, allValues) => value < allValues[otherField] ? value : previousValue;

// const greaterThan = otherField =>
//   (value, previousValue, allValues) => value > allValues[otherField] ? value : previousValue;

class EllipsoidInput extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
    };
  }

  render() {
    const {
      handleSubmit,
      input,
      classes,
    } = this.props;

    return (
      <form className={classes.root} autoComplete="off">
      <div>
            <Tooltip title="semi axis a">
              <TextField
                className={classes.textInput} 
                label="a"
                id="a"
                type="number"
                inputProps={{
                  min: 0.125,
                  step: 0.125,
                }}
                value={input.a}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                  // this.forceUpdate();
                }}
              />
            </Tooltip>

            <Tooltip title="semi axis b">
              <TextField
                className={classes.textInput} 
                label="b"
                id="b"
                type="number"
                inputProps={{
                  min: 0.125,
                  step: 0.125,
                }}
                value={input.b}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>

            <Tooltip title="semi axis c">
              <TextField
                className={classes.textInput} 
                label="c"
                id="c"
                type="number"
                inputProps={{
                  min: 0.125,
                  step: 0.125,
                }}
                value={input.c}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>
        </div>
        <div>
            <Tooltip title="added height at top of open ellipsoid (theta max < 90)">
              <TextField
                className={classes.textInput} 
                label="hTop"
                id="hTop"
                type="number"
                inputProps={{
                  min: 0,
                  step: 0.125,
                }}
                value={input.hTop}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>

            <Tooltip title="added thickness in the middle of the ellipsoid (vertically)">
              <TextField
                className={classes.textInput} 
                label="hMiddle"
                id="hMiddle"
                type="number"
                inputProps={{
                  min: 0,
                  step: 0.125,
                }}
                value={input.hMiddle}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>

            <Tooltip title="added height at the bottom of an open ellipsoid (theta min > -90)">
              <TextField
                className={classes.textInput} 
                label="hBottom"
                id="hBottom"
                type="number"
                inputProps={{
                  min: 0,
                  step: 0.125,
                }}
                value={input.hBottom}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>
        </div>
        <div>
            <Tooltip title="scaling factor put on the hTop ellipse (based on the ellipse at thetaMax)">
              <TextField
                className={classes.textInput} 
                label="hTopFraction"
                id="hTopFraction"
                type="number"
                inputProps={{
                  min: 0.125,
                  max: 2,
                  step: 0.125,
                }}
                value={input.hTopFraction}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>

            <Tooltip title="factor used to shift the hTop ellipse side to side">
              <TextField
                className={classes.textInput} 
                label="hTopShift"
                id="hTopShift"
                type="number"
                inputProps={{
                  min: -5,
                  max: 5,
                  step: 0.125,
                }}
                value={input.hTopShift}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>
        </div>

        <div>
          <Select
            className={classes.textInput} 
            value={input.ppu}
            onChange={(event) => {
              handleSubmit(event.target.name, event.target.value);
            }}
            inputProps={{
              name: 'ppu',
              id: 'ppu-simple',
            }}
          >
            <MenuItem value="">
              <em>None</em>
            </MenuItem>
            <MenuItem value={96}>in</MenuItem>
            <MenuItem value={3.7795276}>mm</MenuItem>
            <MenuItem value={37.795276}>cm</MenuItem>
          </Select>
        </div>

        <div>
            <Tooltip title="Angle defining the bottom of the ellipsoid.  -90 is fully closed on the bottom">
              <TextField
                className={classes.textInput} 
                label="thetaMin"
                id="thetaMin"
                type="number"
                inputProps={{
                  min: -90,
                  max: 85,
                  step: 5,
                }}
                value={input.thetaMin}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value));
                }}
              />
            </Tooltip>

            <Tooltip title="Angle defining the top of the ellipsoid.  90 is fully closed on the top">
              <TextField
                className={classes.textInput} 
                label="thetaMax"
                id="thetaMax"
                type="number"
                inputProps={{
                  min: -85,
                  max: 90,
                  step: 5,
                }}
                value={input.thetaMax}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>
        </div>

        <div>
            <Tooltip title="Number of longitudinal divisions of the ellipsoid.">
              <TextField
                className={classes.textInput} 
                label="Divisions"
                id="Divisions"
                type="number"
                inputProps={{
                  min: 3,
                  max: 100,
                  step: 1,
                }}
                value={input.Divisions}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>

            <Tooltip title="Number of latitudinal divisions of the ellipsoid.">
              <TextField
                className={classes.textInput} 
                label="divisions"
                id="divisions"
                type="number"
                inputProps={{
                  min: 3,
                  max: 100,
                  step: 1,
                }}
                value={input.divisions}
                onChange={(event) => {
                  handleSubmit(event.target.id, parseFloat(event.target.value) );
                }}
              />
            </Tooltip>
        </div>
      </form>
    )
  }
}

const mapStateToProps = state => ({
  input: state.input,
});

const mapDispatchToProps = dispatch => ({
  handleSubmit: (nameInput, valueInput) => dispatch({ type: UPDATE_INPUT, name: nameInput, value: valueInput }),
});
  
export default compose(
  withStyles(styles),
  connect(
    mapStateToProps,
    mapDispatchToProps,
  ),
)(EllipsoidInput);
