import { createRequire } from 'module';
const require = createRequire(import.meta.url);

const { keccak256 } = require("ethereum-cryptography/keccak");
const { utf8ToBytes } = require("ethereum-cryptography/utils");
const SHA256 = require('crypto-js/sha256');

export function hashMessage(message) {
    // Convert message to bytes
    const bytes = utf8ToBytes(message);
    // Hash bytes
    const hash = SHA256(bytes);
    // Return hash
    return `0x${hash}`

}
