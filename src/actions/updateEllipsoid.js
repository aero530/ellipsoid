import { UPDATEELLIPSOID } from './types';

export default function updateEllipsoid(value) {
    return {
        type: UPDATEELLIPSOID,
        payload: value
    }
}