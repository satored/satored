import { doubleBlake3Hash, blake3Hash } from "./blake3";
import { Buffer } from "buffer";
import bs58 from "bs58";
import IsoHex from "./iso-hex";
import PubKey from "./pub-key";
import { Result, Ok, Err } from "./ts-results/result";

// public key hash
export default class Pkh {
  buf: Buffer;

  constructor(pkhBuf: Buffer) {
    this.buf = pkhBuf;
  }

  static fromPubKeyBuf(pubKeyBuf: Buffer): Pkh {
    let pkhBuf = doubleBlake3Hash(pubKeyBuf);
    return new Pkh(pkhBuf);
  }

  static fromPubKey(pubKey: PubKey): Pkh {
    return Pkh.fromPubKeyBuf(pubKey.toIsoBuf());
  }

  static fromIsoBuf(buf: Buffer): Result<Pkh, string> {
    if (buf.length !== 32) {
      return new Err("Invalid public key hash length");
    }
    return new Ok(new Pkh(buf));
  }

  toIsoStr(): string {
    let checkHash = blake3Hash(this.buf).subarray(0, 4);
    let checkHex = checkHash.toString("hex");
    return "ebxpkh" + checkHex + bs58.encode(this.buf);
  }

  static fromIsoStr(pkhStr: string): Result<Pkh, string> {
    if (!pkhStr.startsWith("ebxpkh")) {
      return new Err("Invalid pkh format");
    }
    let checkHex = pkhStr.slice(6, 14);
    let checkBuf = IsoHex.decode(checkHex).unwrap();
    let buf = Buffer.from(bs58.decode(pkhStr.slice(14)));
    let hashBuf = blake3Hash(buf);
    let checkHash = hashBuf.subarray(0, 4);
    if (!checkHash.equals(checkBuf)) {
      return new Err("Invalid pkh checksum");
    }
    return Pkh.fromIsoBuf(buf);
  }

  static isValidStringFmt(pkhStr: string): boolean {
    let pkh = Pkh.fromIsoStr(pkhStr);
    return pkh.ok;
  }
}
