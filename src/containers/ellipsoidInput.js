import React from 'react';
//import { connect } from 'react-redux'
import { Field, reduxForm } from 'redux-form';

// initial values are numbers here.  when input comes from the form it comes in as a string.  the strings are parsed by in ellipsoid.js
const initialValues = {
    a : 3.75, //in - major axis radius
    b : 2.875, //in - minor axis radius
    c : 3, //in - height axis radius
    hTop : 0, //in - height added to bottom of ellipsoid
    hMiddle : 2,
    hBottom : 2,
    hTopFraction : 1.0,
    hTopShift : 0,
    Divisions : 8, // divisions around major / minor direction
    divisions : 16, // divisions in height
    ppu : 96, //pixels per unit (in)  This is the standard ppi for inkscape
    imageOffset : 0.5, //in
    minGap : 0.001, //in
    thetaMin : -35,
    thetaMax : 90,
    projection : 'cylindrical', // circular or cylindrical
  }


const EllipsoidInput = (props) => {
    const { handleSubmit, pristine, reset, submitting } = props
    return (
      <form onSubmit={handleSubmit}>
      <div>
            <label>a </label>
            <Field name="a" component="input" type="number" min="0" step="0.125"/>

            <label> b </label>
            <Field name="b" component="input" type="number" min="0" step="0.125"/>

            <label> c </label>
            <Field name="c" component="input" type="number" min="0" step="0.125"/>
        </div>
        <div>
            <label>h top </label>
            <Field name="hTop" component="input" type="number" min="0" step="0.125"/>

            <label> h middle </label>
            <Field name="hMiddle" component="input" type="number" min="0" step="0.125"/>

            <label> h bottom </label>
            <Field name="hBottom" component="input" type="number" min="0" step="0.125"/>
        </div>
        <div>
            <label>h top diam fraction </label>
            <Field name="hTopFraction" component="input" type="number" min="0" max="2" step="0.125"/>

            <label> h top shift </label>
            <Field name="hTopShift" component="input" type="number" min="-5" max="5" step="0.125"/>
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
            <Field name="thetaMin" component="input" type="number" min="-90" max="0" step="5"/>

            <label> theta max </label>
            <Field name="thetaMax" component="input" type="number" min="0" max="90" step="5"/>
        </div>

        <div>
            <label>Divisions </label>
            <Field name="Divisions" component="input" type="number" min="0" max="100" step="1"/>

            <label> divisions </label>
            <Field name="divisions" component="input" type="number" min="0" max="100" step="1"/>
        </div>
        <div>
            <label>image offset </label>
            <Field name="imageOffset" component="input" type="number" min="0" step="0.25"/>

            <label> minimum line gap </label>
            <Field name="minGap" component="input" type="number" min="0" step="0.001"/>
        </div>

        <div>
            <label>Projection </label>
            <label><Field name="projection" component="input" type="radio" value="spherical"/> spherical</label>
            <label><Field name="projection" component="input" type="radio" value="cylindrical"/> cylindrical</label>
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