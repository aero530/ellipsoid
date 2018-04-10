import { PROJECTIONCHANGED } from './types';

export default function projectionChanged(value) {
    return {
        type: PROJECTIONCHANGED,
        payload: value
    }
}