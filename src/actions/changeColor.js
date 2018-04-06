import { CHANGECOLOR } from './types';

export default function changeColor(value) {
    return {
        type: CHANGECOLOR,
        payload: value
    }
}