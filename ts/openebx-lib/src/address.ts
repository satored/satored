import { doubleBlake3Hash } from './blake3'
import { Buffer } from 'buffer'

export default class Address {
  private _address: Uint8Array

  constructor(publicKey: Uint8Array) {
    this._address = doubleBlake3Hash(publicKey)
  }

  get address(): Uint8Array {
    return this._address
  }
}