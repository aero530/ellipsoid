import { DECREMENT } from './types';


export default function decrementCounter(value) {
    return {
        type: DECREMENT,
        payload: value
    }
}