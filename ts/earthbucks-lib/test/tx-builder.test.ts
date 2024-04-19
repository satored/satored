import { describe, expect, test, beforeEach, it } from '@jest/globals'
import TxBuilder from '../src/tx-builder'
import TxOutputMap from '../src/tx-output-map'
import TxOutput from '../src/tx-output'
import Script from '../src/script'
import Key from '../src/key'
import Pkh from '../src/pkh'
import PkhKeyMap from '../src/pkh-key-map'
import { Buffer } from 'buffer'

describe('TxBuilder', () => {
  let txBuilder: TxBuilder
  let txOutMap: TxOutputMap
  let pkhKeyMap: PkhKeyMap

  beforeEach(() => {
    txOutMap = new TxOutputMap()
    pkhKeyMap = new PkhKeyMap()
    // generate 5 keys, 5 outputs, and add them to the txOutMap
    for (let i = 0; i < 5; i++) {
      const key = Key.fromRandom()
      const pkh = new Pkh(key.publicKey)
      pkhKeyMap.add(key, pkh.pkh)
      const script = Script.fromAddressOutput(pkh.pkh)
      const output = new TxOutput(BigInt(100), script)
      txOutMap.add(output, Buffer.from('00'.repeat(32), 'hex'), i)
    }

    const changeScript = Script.fromString('')
    txBuilder = new TxBuilder(txOutMap, changeScript)
  })

  test('should build a valid tx when input is enough to cover the output', () => {
    const key = Key.fromRandom()
    const pkh = new Pkh(key.publicKey)
    const script = Script.fromAddressOutput(pkh.pkh)
    const output = new TxOutput(BigInt(50), script)
    txBuilder.addOutput(BigInt(50), Script.fromString(''))

    const tx = txBuilder.build()

    expect(tx.inputs.length).toBe(1)
    expect(tx.outputs.length).toBe(2)
    expect(tx.outputs[0].value).toBe(BigInt(50))
  })

  test('should build an invalid tx when input is insufficient to cover the output', () => {
    txBuilder.addOutput(BigInt(10000), Script.fromString(''))

    const tx = txBuilder.build()

    expect(tx.inputs.length).toBe(5)
    expect(tx.outputs.length).toBe(1)
    expect(txBuilder.inputAmount).toBe(BigInt(500))
    expect(tx.outputs[0].value).toBe(BigInt(10000))
  })
})