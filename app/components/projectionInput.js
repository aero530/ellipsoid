import React from 'react';
import compose from 'recompose/compose';
import { connect } from 'react-redux';
import { withStyles } from '@material-ui/core/styles';
import TextField from '@material-ui/core/TextField';
import Tooltip from '@material-ui/core/Tooltip';
import Radio from '@material-ui/core/Radio';
import RadioGroup from '@material-ui/core/RadioGroup';

import FormGroup from '@material-ui/core/FormGroup';
import FormControlLabel from '@material-ui/core/FormControlLabel';
import FormControl from '@material-ui/core/FormControl';
import FormLabel from '@material-ui/core/FormLabel';

import Switch from '@material-ui/core/Switch';

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

class ProjectionInput extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
    };
  }

  render() {
    const {
      classes,
      input,
      handleSubmit,
    } = this.props;

    return (
      <form>
        <div>
          <Tooltip title="Padding the SVG around the ellipsoid pattern">
            <TextField
              className={classes.textInput} 
              label="imageOffset"
              id="imageOffset"
              type="number"
                inputProps={{
                  min: 0,
                  step: 0.25,
                }}
              value={input.imageOffset}
              onChange={(event) => {
                handleSubmit(event.target.id, parseFloat(event.target.value) );
              }}
            />
          </Tooltip>
          <Tooltip title="Minimum gap allowed between lines in the SVG image.  Helpful for allowing for cutting tool radius.">
            <TextField
              className={classes.textInput} 
              label="minGap"
              id="minGap"
              type="number"
                inputProps={{
                  min: 0,
                  step: 0.001,
                }}
              value={input.minGap}
              onChange={(event) => {
                handleSubmit(event.target.id, parseFloat(event.target.value) );
              }}
            />
          </Tooltip>
        </div>
        <div>
          <Tooltip title="Pattern projection type.  Spherical = Unfold from the top of the ellipsoid.  Cylindrical = unfold from the front of the ellipsoid.">
            <FormControl component="fieldset" className={classes.formControl}>
              <FormLabel component="legend">Projection Type</FormLabel>
              <RadioGroup
                className={classes.textInput} 
                aria-label="Projection Type"
                name="projection"
                className={classes.group}
                value={input.projection}
                onChange={(event) => {
                  handleSubmit(event.target.name, event.target.value);
                }}
              >
              
                <FormControlLabel value="spherical" control={<Radio />} label="spherical" />
                <FormControlLabel value="cylindrical" control={<Radio />} label="cylindrical" />
              </RadioGroup>
            </FormControl>
        </Tooltip>
        </div>
        <Tooltip title="Save SVG as Inkscape file with layers.  Turn off for plain SVG.">
        <FormGroup row>
          <FormControlLabel
            control={
              
              <Switch
                checked={input.inkscapeLayers}
                id="inkscapeLayers"
                onChange={(event) => {
                  handleSubmit(event.target.id, event.target.checked);
                }}
                value="inkscapeLayers"
              />
              }
            label="Inkscape Layers"
          />
        </FormGroup>
        </Tooltip>
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
)(ProjectionInput);
