import { PrivKey } from "./priv-key.js";
import { PubKey } from "./pub-key.js";
import { EbxBuf } from "./ebx-buf.js";
import { Result, Ok, Err } from "earthbucks-opt-res";

export class KeyPair {
  privKey: PrivKey;
  pubKey: PubKey;

  constructor(privKey: PrivKey, pubKey: PubKey) {
    this.privKey = privKey;
    this.pubKey = pubKey;
  }

  static fromPrivKey(privKey: PrivKey): KeyPair {
    const pubKey = PubKey.fromPrivKey(privKey).unwrap();
    return new KeyPair(privKey, pubKey);
  }

  static fromPrivKeyIsoBuf(privKeyBuf: EbxBuf): Result<KeyPair, string> {
    const privKeyRes = PrivKey.fromIsoBuf(EbxBuf.from(privKeyBuf)).mapErr(
      (err) => "Error parsing private key: " + err,
    );
    if (privKeyRes.err) {
      return privKeyRes;
    }
    const privKey = privKeyRes.unwrap();
    const pubKey = PubKey.fromPrivKey(privKey).unwrap();
    return Ok(new KeyPair(privKey, pubKey));
  }

  static fromRandom(): KeyPair {
    const privKey = PrivKey.fromRandom();
    return KeyPair.fromPrivKey(privKey);
  }
}
