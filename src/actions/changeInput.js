import { CHANGEINPUT } from './types';

export default function changeInput(value) {
    return {
        type: CHANGEINPUT,
        payload: value
    }
}