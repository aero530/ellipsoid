import React from 'react'
import EllipsoidInput from '../containers/ellipsoidInput'

class InputForm extends React.Component {
  submit = values => {
    // print the form values to the console
    console.log(values)
  }
  render() {
    return <EllipsoidInput onSubmit={this.submit} />
  }
}

export default InputForm