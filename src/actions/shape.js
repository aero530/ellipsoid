import { UPDATEGEOMETRY } from './types';
import { UPDATEPATTERN } from './types';

export function updateGeometry(value) {
  return {
    type: UPDATEGEOMETRY,
    payload: value
  }
}

export function updatePattern(value) {
  return {
    type: UPDATEPATTERN,
    payload: value
  }
}