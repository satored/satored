import { GenericError } from "./ebx-error.js";
import * as Hash from "./hash.js";
import { SysBuf, FixedIsoBuf } from "./iso-buf.js";

export class MerkleNode {
  public left: MerkleNode | null;
  public right: MerkleNode | null;
  public hashedData: FixedIsoBuf<32>;

  constructor(
    left: MerkleNode | null,
    right: MerkleNode | null,
    hashedData: FixedIsoBuf<32>,
  ) {
    this.left = left;
    this.right = right;
    this.hashedData = hashedData;
  }

  public hash(): FixedIsoBuf<32> {
    if (this.left || this.right) {
      const leftData = this.left ? this.left.hash() : SysBuf.alloc(0);
      const rightData = this.right ? this.right.hash() : leftData;
      return Hash.doubleBlake3Hash(SysBuf.concat([leftData, rightData]));
    } else {
      return this.hashedData;
    }
  }

  static fromIsoBufs(hashedDatas: FixedIsoBuf<32>[]): MerkleNode {
    if (hashedDatas.length === 0) {
      throw new GenericError("Cannot create MerkleNode from empty array");
    }
    if (hashedDatas.length === 1) {
      return new MerkleNode(null, null, hashedDatas[0]);
    }
    if (hashedDatas.length === 2) {
      const left = new MerkleNode(null, null, hashedDatas[0]);
      const right = new MerkleNode(null, null, hashedDatas[1]);
      return new MerkleNode(
        left,
        right,
        Hash.doubleBlake3Hash(SysBuf.concat([left.hash(), right.hash()])),
      );
    }
    // Make sure the number of elements is a power of two
    while ((hashedDatas.length & (hashedDatas.length - 1)) !== 0) {
      hashedDatas.push(hashedDatas[hashedDatas.length - 1]);
    }
    const left = MerkleNode.fromIsoBufs(
      hashedDatas.slice(0, hashedDatas.length / 2),
    );
    const right = MerkleNode.fromIsoBufs(
      hashedDatas.slice(hashedDatas.length / 2),
    );

    return new MerkleNode(
      left,
      right,
      Hash.doubleBlake3Hash(SysBuf.concat([left.hash(), right.hash()])),
    );
  }
}
