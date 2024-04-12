import Tx from './tx'
import Key from './key'
import Pkh from './pkh'
import PkhKeyMap from './pkh-key-map'
import TxOutputMap from './tx-output-map'
import TxSignature from './tx-signature'
import { Buffer } from 'buffer'

export default class TxSigner {
  public tx: Tx
  public pkhKeyMap: PkhKeyMap
  public txOutMap: TxOutputMap

  constructor(tx: Tx, txOutMap: TxOutputMap, pkhKeyMap: PkhKeyMap) {
    this.tx = tx
    this.txOutMap = txOutMap
    this.pkhKeyMap = pkhKeyMap
  }

  sign(nIn: number): boolean {
    const txInput = this.tx.inputs[nIn]
    const txOutHash = txInput.inputTxId
    const outputIndex = txInput.inputTxNOut
    const txOut = this.txOutMap.get(txOutHash, outputIndex)
    if (!txOut) {
      return false
    }
    if (!txOut.script.isAddressOutput()) {
      return false
    }
    const pkh = txOut.script.chunks[2].buffer as Uint8Array
    const inputScript = txInput.script
    if (!inputScript.isAddressInput()) {
      return false
    }
    const key = this.pkhKeyMap.get(pkh)
    if (!key) {
      return false
    }
    const pubKey = key.publicKey
    if (pubKey.length !== 33) {
      return false
    }
    inputScript.chunks[1].buffer = Buffer.from(pubKey)
    const outputScriptBuf = txOut.script.toU8Vec()
    const outputAmount = txOut.value
    const sig = this.tx.signNoCache(
      nIn,
      key.privateKey,
      outputScriptBuf,
      outputAmount,
      TxSignature.SIGHASH_ALL,
    )
    const sigBuf = sig.toU8Vec()
    if (sigBuf.length !== 65) {
      return false
    }
    inputScript.chunks[0].buffer = Buffer.from(sigBuf)
    txInput.script = inputScript
    return true
  }

  signAll(): boolean {
    for (let i = 0; i < this.tx.inputs.length; i++) {
      if (!this.sign(i)) {
        return false
      }
    }
    return true
  }
}