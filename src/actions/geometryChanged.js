import { GEOMETRYCHANGED } from './types';

export default function geometryChanged(value) {
    return {
        type: GEOMETRYCHANGED,
        payload: value
    }
}