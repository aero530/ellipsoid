import { GEOMETRYCHANGED } from './types';
import { PROJECTIONCHANGED } from './types';

export function projectionChanged(value) {
    return {
        type: PROJECTIONCHANGED,
        payload: value
    }
}

export function geometryChanged(value) {
    return {
        type: GEOMETRYCHANGED,
        payload: value
    }
}