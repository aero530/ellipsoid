import { INCREMENT } from './types';

export default function incrementCounter(value) {
    return {
        type: INCREMENT,
        payload: value
    }
}