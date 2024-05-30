import { Header } from "./header.js";
import { Tx } from "./tx.js";
import { IsoBufWriter } from "./iso-buf-writer.js";
import { IsoBufReader } from "./iso-buf-reader.js";
import { EbxBuf } from "./ebx-buf.js";
import { Result, Ok, Err } from "earthbucks-opt-res";

export class Block {
  public header: Header;
  public txs: Tx[];

  constructor(header: Header, txs: Tx[]) {
    this.header = header;
    this.txs = txs;
  }

  static fromIsoBufReader(br: IsoBufReader): Result<Block, string> {
    const headerRes = Header.fromIsoBufReader(br).mapErr(
      (err) => `Unable to parse header: ${err}`,
    );
    if (headerRes.err) {
      return headerRes;
    }
    const header = headerRes.unwrap();
    const txCountRes = br
      .readVarInt()
      .mapErr((err) => `Unable to parse tx count: ${err}`);
    if (txCountRes.err) {
      return txCountRes;
    }
    const txCount = txCountRes.unwrap();

    const txs: Tx[] = [];
    for (let i = 0; i < txCount; i++) {
      const txRes = Tx.fromIsoBufReader(br).mapErr(
        (err) => `Unable to parse tx ${i}: ${err}`,
      );
      if (txRes.err) {
        return txRes;
      }
      const tx = txRes.unwrap();
      txs.push(tx);
    }

    return Ok(new Block(header, txs));
  }

  toIsoBufWriter(bw: IsoBufWriter): IsoBufWriter {
    bw.writeIsoBuf(this.header.toIsoBuf());
    bw.writeVarIntNum(this.txs.length);
    this.txs.forEach((tx) => {
      bw.writeIsoBuf(tx.toIsoBuf());
    });
    return bw;
  }

  toIsoBuf(): EbxBuf {
    return this.toIsoBufWriter(new IsoBufWriter()).toIsoBuf();
  }

  static fromIsoBuf(buf: EbxBuf): Result<Block, string> {
    return Block.fromIsoBufReader(new IsoBufReader(buf));
  }

  isGenesis(): boolean {
    return this.header.isGenesis() && this.txs.length === 1;
  }
}
