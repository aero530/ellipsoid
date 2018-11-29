import React from 'react';
import { Field, reduxForm } from 'redux-form';

// initial values are numbers here.  when input comes from the form it comes in as a string.  the strings are parsed by in ellipsoid.js
const initialValues = {
  a: 3.75, //in - major axis radius
  b: 2.875, //in - minor axis radius
  c: 3, //in - height axis radius
  hTop: 0, //in - height added to bottom of ellipsoid
  hMiddle: 2,
  hBottom: 2,
  hTopFraction: 1.0,
  hTopShift: 0,
  Divisions: 8, // divisions around major / minor direction
  divisions: 16, // divisions in height
  ppu: 96, //pixels per unit (in)  This is the standard ppi for inkscape
  thetaMin: -35,
  thetaMax: 90,
};

const lessThan = otherField =>
  (value, previousValue, allValues) => value < allValues[otherField] ? value : previousValue;

const greaterThan = otherField =>
  (value, previousValue, allValues) => value > allValues[otherField] ? value : previousValue;

const EllipsoidInput = (props) => {
  const {
    handleSubmit,
    pristine,
    reset,
    submitting,
  } = props;

  return (
    <form onSubmit={handleSubmit}>
    <div>
          <label>a </label>
          <Field name="a" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />

          <label> b </label>
          <Field name="b" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />

          <label> c </label>
          <Field name="c" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />
      </div>
      <div>
          <label>h theta max</label>
          <Field name="hTop" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />

          <label> h theta=0 </label>
          <Field name="hMiddle" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />

          <label> h theta min </label>
          <Field name="hBottom" component="input" type="number" min="0" step="0.125" parse={value => Number(value)} />
      </div>
      <div>
          <label>h top diam fraction </label>
          <Field name="hTopFraction" component="input" type="number" min="0" max="2" step="0.125" parse={value => Number(value)} />

          <label> h top shift </label>
          <Field name="hTopShift" component="input" type="number" min="-5" max="5" step="0.125" parse={value => Number(value)} />
      </div>

      <div>
        <label>units </label>
        <div>
          <Field name="ppu" component="select">
            <option></option>
            <option value={96}>in</option>
            <option value={3.7795276}>mm</option>
            <option value={37.795276}>cm</option>
          </Field>
        </div>
      </div>

      <div>
          <label>theta min </label>
          <Field name="thetaMin" component="input" type="number" min="-90" max="85" step="5" normalize={lessThan('thetaMax')} parse={value => Number(value)} />

          <label> theta max </label>
          <Field name="thetaMax" component="input" type="number" min="-85" max="90" step="5" normalize={greaterThan('thetaMin')} parse={value => Number(value)} />
      </div>

      <div>
          <label>Divisions </label>
          <Field name="Divisions" component="input" type="number" min="5" max="100" step="1" parse={value => Number(value)} />

          <label> divisions </label>
          <Field name="divisions" component="input" type="number" min="3" max="100" step="1" parse={value => Number(value)} />
      </div>

      <div>
        <button type="submit" disabled={pristine || submitting}>Submit</button>
        <button type="button" disabled={pristine || submitting} onClick={reset}>Default Values</button>
      </div>
    </form>
  )
}


export default reduxForm({
    form: 'EllipsoidInput',  // a unique identifier for this form
    initialValues: initialValues
  })(EllipsoidInput)
