import React from 'react';
import { Field, reduxForm } from 'redux-form';


// initial values are numbers here.  when input comes from the form it comes in as a string.  the strings are parsed by in ellipsoid.js
const initialValues = {
    imageOffset : 0.5, //in
    minGap : 0.001, //in
    projection : 'cylindrical', // circular or cylindrical
  }


const ProjectionInput = (props) => {
    return (
      <form>
        <div>
            <label>image offset </label>
            <Field name="imageOffset" component="input" type="number" min="0" step="0.25" parse={value => Number(value)} />

            <label> minimum line gap </label>
            <Field name="minGap" component="input" type="number" min="0" step="0.001" parse={value => Number(value)} />
        </div>

        <div>
            <label>Projection </label>
            <label><Field name="projection" component="input" type="radio" value="spherical"/> spherical</label>
            <label><Field name="projection" component="input" type="radio" value="cylindrical"/> cylindrical</label>
        </div>
      </form>
    )
  }


export default reduxForm({
    form: 'ProjectionInput',  // a unique identifier for this form
    initialValues: initialValues
  })(ProjectionInput)