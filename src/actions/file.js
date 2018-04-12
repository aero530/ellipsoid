import { FILEIMPORT } from './types';

export default function fileImport(value) {
    return {
        type: FILEIMPORT,
        payload: value
    }
}