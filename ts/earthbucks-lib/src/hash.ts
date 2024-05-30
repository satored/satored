import { hash, createKeyed } from "blake3";
import { EbxBuf } from "./ebx-buf";
import { blake3 as blake3browser } from "@noble/hashes/blake3";

type IsoBufFunction = (input: EbxBuf) => EbxBuf;
type MacFunction = (key: EbxBuf, data: EbxBuf) => EbxBuf;

let blake3Hash: IsoBufFunction;
let doubleBlake3Hash: IsoBufFunction;
let blake3Mac: MacFunction;

if (typeof document === "undefined") {
  // running in a server environment
  blake3Hash = function blake3Hash(data: EbxBuf): EbxBuf {
    return hash(data) as EbxBuf;
  };

  doubleBlake3Hash = function doubleBlake3Hash(data: EbxBuf): EbxBuf {
    return blake3Hash(blake3Hash(data));
  };

  blake3Mac = function blake3Mac(key: EbxBuf, data: EbxBuf): EbxBuf {
    return createKeyed(key).update(data).digest() as EbxBuf;
  };
} else {
  blake3Hash = function blake3Hash(data: EbxBuf): EbxBuf {
    return EbxBuf.from(blake3browser(data));
  };

  doubleBlake3Hash = function doubleBlake3Hash(data: EbxBuf): EbxBuf {
    return blake3Hash(blake3Hash(data));
  };

  blake3Mac = function blake3Mac(key: EbxBuf, data: EbxBuf): EbxBuf {
    return EbxBuf.from(blake3browser(data, { key: key }));
  };
}

export { blake3Hash, doubleBlake3Hash, blake3Mac };
