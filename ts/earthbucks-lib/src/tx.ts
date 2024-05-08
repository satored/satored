import TxInput from "./tx-input";
import TxOutput from "./tx-output";
import VarInt from "./var-int";
import IsoBufReader from "./iso-buf-reader";
import IsoBufWriter from "./iso-buf-writer";
import { blake3Hash, doubleBlake3Hash } from "./blake3";
import secp256k1 from "secp256k1";
const { ecdsaSign, ecdsaVerify } = secp256k1;
import TxSignature from "./tx-signature";
import Script from "./script";
import { Buffer } from "buffer";
import { Result, Ok, Err } from "./ts-results/result";
import IsoHex from "./iso-hex";

export class HashCache {
  public hashPrevouts?: Buffer;
  public hashSequence?: Buffer;
  public hashOutputs?: Buffer;
}

export default class Tx {
  public version: number;
  public inputs: TxInput[];
  public outputs: TxOutput[];
  public lockNum: bigint;

  constructor(
    version: number,
    inputs: TxInput[],
    outputs: TxOutput[],
    lockNum: bigint,
  ) {
    this.version = version;
    this.inputs = inputs;
    this.outputs = outputs;
    this.lockNum = lockNum;
  }

  static fromIsoBuf(buf: Buffer): Result<Tx, string> {
    return Tx.fromIsoBufReader(new IsoBufReader(buf));
  }

  static fromIsoBufReader(reader: IsoBufReader): Result<Tx, string> {
    try {
      const version = reader.readUInt8().unwrap();
      const numInputs = reader.readVarIntNum().unwrap();
      const inputs = [];
      for (let i = 0; i < numInputs; i++) {
        inputs.push(TxInput.fromIsoBufReader(reader));
      }
      const numOutputs = reader.readVarIntNum().unwrap();
      const outputs = [];
      for (let i = 0; i < numOutputs; i++) {
        outputs.push(TxOutput.fromIsoBufReader(reader));
      }
      const lockNum = reader.readUInt64BE().unwrap();
      return new Ok(new Tx(version, inputs, outputs, BigInt(lockNum)));
    } catch (err) {
      return new Err(err?.toString() || "Unknown error parsing tx");
    }
  }

  toIsoBuf(): Buffer {
    const writer = new IsoBufWriter();
    writer.writeUInt8(this.version);
    writer.writeBuffer(VarInt.fromNumber(this.inputs.length).toIsoBuf());
    for (const input of this.inputs) {
      writer.writeBuffer(input.toIsoBuf());
    }
    writer.writeBuffer(VarInt.fromNumber(this.outputs.length).toIsoBuf());
    for (const output of this.outputs) {
      writer.writeBuffer(output.toIsoBuf());
    }
    writer.writeUInt64BE(this.lockNum);
    return writer.toIsoBuf();
  }

  toIsoHex(): string {
    return this.toIsoBuf().toString("hex");
  }

  static fromIsoHex(hex: string): Result<Tx, string> {
    try {
      const buf = IsoHex.decode(hex)
        .mapErr((err) => `Could not decode hex: ${err}`)
        .unwrap();
      return Tx.fromIsoBuf(buf);
    } catch (err) {
      return new Err(err?.toString() || "Unknown error parsing hex tx");
    }
  }

  static fromCoinbase(
    inputScript: Script,
    outputScript: Script,
    outputAmount: bigint,
  ): Tx {
    const version = 1;
    const inputs = [TxInput.fromCoinbase(inputScript)];
    const outputs = [new TxOutput(outputAmount, outputScript)];
    const lockNum = BigInt(0);
    return new Tx(version, inputs, outputs, lockNum);
  }

  isCoinbase(): boolean {
    return this.inputs.length === 1 && this.inputs[0].isCoinbase();
  }

  blake3Hash(): Buffer {
    return blake3Hash(this.toIsoBuf());
  }

  id(): Buffer {
    return doubleBlake3Hash(this.toIsoBuf());
  }

  hashPrevouts(): Buffer {
    const writer = new IsoBufWriter();
    for (const input of this.inputs) {
      writer.writeBuffer(input.inputTxId);
      writer.writeUInt32BE(input.inputTxNOut);
    }
    return doubleBlake3Hash(writer.toIsoBuf());
  }

  hashSequence(): Buffer {
    const writer = new IsoBufWriter();
    for (const input of this.inputs) {
      writer.writeUInt32BE(input.sequence);
    }
    return doubleBlake3Hash(writer.toIsoBuf());
  }

  hashOutputs(): Buffer {
    const writer = new IsoBufWriter();
    for (const output of this.outputs) {
      writer.writeBuffer(output.toIsoBuf());
    }
    return doubleBlake3Hash(writer.toIsoBuf());
  }

  sighashPreimage(
    inputIndex: number,
    script: Buffer,
    amount: bigint,
    hashType: number,
    hashCache: HashCache,
  ): Buffer {
    const SIGHASH_ANYONECANPAY = 0x80;
    const SIGHASH_SINGLE = 0x03;
    const SIGHASH_NONE = 0x02;

    let prevoutsHash = Buffer.alloc(32);
    let sequenceHash = Buffer.alloc(32);
    let outputsHash = Buffer.alloc(32);

    if (!(hashType & SIGHASH_ANYONECANPAY)) {
      if (!hashCache.hashPrevouts) {
        hashCache.hashPrevouts = this.hashPrevouts();
      }
      prevoutsHash = hashCache.hashPrevouts;
    }

    if (
      !(hashType & SIGHASH_ANYONECANPAY) &&
      (hashType & 0x1f) !== SIGHASH_SINGLE &&
      (hashType & 0x1f) !== SIGHASH_NONE
    ) {
      if (!hashCache.hashSequence) {
        hashCache.hashSequence = this.hashSequence();
      }
      sequenceHash = hashCache.hashSequence;
    }

    if (
      (hashType & 0x1f) !== SIGHASH_SINGLE &&
      (hashType & 0x1f) !== SIGHASH_NONE
    ) {
      if (!hashCache.hashOutputs) {
        hashCache.hashOutputs = this.hashOutputs();
      }
      outputsHash = hashCache.hashOutputs;
    } else if (
      (hashType & 0x1f) === SIGHASH_SINGLE &&
      inputIndex < this.outputs.length
    ) {
      outputsHash = doubleBlake3Hash(this.outputs[inputIndex].toIsoBuf());
    }

    const writer = new IsoBufWriter();
    writer.writeUInt8(this.version);
    writer.writeBuffer(prevoutsHash);
    writer.writeBuffer(sequenceHash);
    writer.writeBuffer(this.inputs[inputIndex].inputTxId);
    writer.writeUInt32BE(this.inputs[inputIndex].inputTxNOut);
    writer.writeVarIntNum(script.length);
    writer.writeBuffer(script);
    writer.writeUInt64BE(amount);
    writer.writeUInt32BE(this.inputs[inputIndex].sequence);
    writer.writeBuffer(outputsHash);
    writer.writeUInt64BE(this.lockNum);
    writer.writeUInt8(hashType);
    return writer.toIsoBuf();
  }

  sighashNoCache(
    inputIndex: number,
    script: Buffer,
    amount: bigint,
    hashType: number,
  ): Buffer {
    const hashCache = new HashCache();
    const preimage = this.sighashPreimage(
      inputIndex,
      script,
      amount,
      hashType,
      hashCache,
    );
    let hash = doubleBlake3Hash(preimage);
    return hash;
  }

  sighashWithCache(
    inputIndex: number,
    script: Buffer,
    amount: bigint,
    hashType: number,
    hashCache: HashCache,
  ): Buffer {
    const preimage = this.sighashPreimage(
      inputIndex,
      script,
      amount,
      hashType,
      hashCache,
    );
    let hash = doubleBlake3Hash(preimage);
    return hash;
  }

  signNoCache(
    inputIndex: number,
    privateKey: Buffer,
    script: Buffer,
    amount: bigint,
    hashType: number,
  ): TxSignature {
    const hash = this.sighashNoCache(inputIndex, script, amount, hashType);
    let sigBuf = Buffer.from(ecdsaSign(hash, privateKey).signature);
    const sig = new TxSignature(hashType, sigBuf);
    return sig;
  }

  signWithCache(
    inputIndex: number,
    privateKey: Buffer,
    script: Buffer,
    amount: bigint,
    hashType: number,
    hashCache: HashCache,
  ): TxSignature {
    const hash = this.sighashWithCache(
      inputIndex,
      script,
      amount,
      hashType,
      hashCache,
    );
    let sigBuf = Buffer.from(ecdsaSign(hash, privateKey).signature);
    const sig = new TxSignature(hashType, sigBuf);
    return sig;
  }

  verifyNoCache(
    inputIndex: number,
    publicKey: Buffer,
    sig: TxSignature,
    script: Buffer,
    amount: bigint,
  ): boolean {
    const hashType = sig.hashType;
    const hash = this.sighashNoCache(inputIndex, script, amount, hashType);
    return ecdsaVerify(sig.sigBuf, hash, publicKey);
  }

  verifyWithCache(
    inputIndex: number,
    publicKey: Buffer,
    sig: TxSignature,
    script: Buffer,
    amount: bigint,
    hashCache: HashCache,
  ): boolean {
    const hashType = sig.hashType;
    const hash = this.sighashWithCache(
      inputIndex,
      script,
      amount,
      hashType,
      hashCache,
    );
    return ecdsaVerify(sig.sigBuf, hash, publicKey);
  }
}
