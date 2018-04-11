import { UPDATEGEOMETRY } from './types';

export default function updateGeometry(value) {
    return {
        type: UPDATEGEOMETRY,
        payload: value
    }
}