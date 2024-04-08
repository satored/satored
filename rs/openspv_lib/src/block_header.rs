use std::time::SystemTime;

use crate::buffer_reader::BufferReader;
use crate::buffer_writer::BufferWriter;

pub struct BlockHeader {
    pub version: u32,                 // uint32
    pub prev_block_id: Vec<u8>, // 256 bits
    pub merkle_root: Vec<u8>,         // 256 bits
    pub timestamp: u64,               // uint32
    pub target: Vec<u8>,              // 32 bits
    pub nonce: Vec<u8>,               // 256 bits
    pub index: u64,                   // uint64
}

impl BlockHeader {
    pub fn new(
        version: u32,
        prev_block_id: Vec<u8>,
        merkle_root: Vec<u8>,
        timestamp: u64,
        target: Vec<u8>,
        nonce: Vec<u8>,
        index: u64,
    ) -> BlockHeader {
        BlockHeader {
            version,
            prev_block_id,
            merkle_root,
            timestamp,
            target,
            nonce,
            index,
        }
    }

    pub fn to_u8_vec(&self) -> Vec<u8> {
        let mut bw = BufferWriter::new();
        bw.write_u32_be(self.version);
        bw.write_u8_vec(self.prev_block_id.clone());
        bw.write_u8_vec(self.merkle_root.clone());
        bw.write_u64_be(self.timestamp);
        bw.write_u8_vec(self.target.clone());
        bw.write_u8_vec(self.nonce.clone());
        bw.write_u64_be(self.index);
        bw.to_u8_vec()
    }

    pub fn from_u8_vec(buf: Vec<u8>) -> Result<BlockHeader, &'static str> {
        let mut br = BufferReader::new(buf);
        let version = br.read_u32_be();
        let previous_block_hash = br.read_u8_vec(32);
        let merkle_root = br.read_u8_vec(32);
        let timestamp = br.read_u64_be();
        let target = br.read_u8_vec(32);
        let nonce = br.read_u8_vec(32);
        let index = br.read_u64_be();
        Ok(BlockHeader::new(
            version,
            previous_block_hash,
            merkle_root,
            timestamp,
            target,
            nonce,
            index,
        ))
    }

    pub fn from_buffer_reader(br: &mut BufferReader) -> BlockHeader {
        let version = br.read_u32_be();
        let previous_block_hash = br.read_u8_vec(32);
        let merkle_root = br.read_u8_vec(32);
        let timestamp = br.read_u64_be();
        let target = br.read_u8_vec(32);
        let nonce = br.read_u8_vec(32);
        let index = br.read_u64_be();
        BlockHeader::new(
            version,
            previous_block_hash,
            merkle_root,
            timestamp,
            target,
            nonce,
            index,
        )
    }

    pub fn to_buffer_writer(&self) -> BufferWriter {
        let mut bw = BufferWriter::new();
        bw.write_u32_be(self.version);
        bw.write_u8_vec(self.prev_block_id.clone());
        bw.write_u8_vec(self.merkle_root.clone());
        bw.write_u64_be(self.timestamp);
        bw.write_u8_vec(self.target.clone());
        bw.write_u8_vec(self.nonce.clone());
        bw.write_u64_be(self.index);
        bw
    }

    pub fn to_string(&self) -> String {
        hex::encode(&self.to_u8_vec())
    }

    pub fn from_string(str: String) -> Result<BlockHeader, &'static str> {
        let buf = hex::decode(str).map_err(|_| "Invalid hex string")?;
        BlockHeader::from_u8_vec(buf)
    }

    pub fn is_valid_version(version: u32) -> bool {
        version == 1
    }

    pub fn is_valid_previous_block_hash(previous_block_hash: Vec<u8>) -> bool {
        previous_block_hash.len() == 32
    }

    pub fn is_valid_merkle_root(merkle_root: Vec<u8>) -> bool {
        merkle_root.len() == 32
    }
    pub fn is_valid_nonce(nonce: Vec<u8>) -> bool {
        nonce.len() == 32
    }

    pub fn is_valid(&self) -> bool {
        let len = self.to_u8_vec().len();
        if len != 148 {
            return false;
        }
        return BlockHeader::is_valid_version(self.version)
            && BlockHeader::is_valid_previous_block_hash(self.prev_block_id.clone())
            && BlockHeader::is_valid_merkle_root(self.merkle_root.clone())
            && BlockHeader::is_valid_nonce(self.nonce.clone());
    }

    pub fn is_genesis(&self) -> bool {
        self.index == 0 && self.prev_block_id.iter().all(|&x| x == 0)
    }
    
    pub fn from_genesis(initial_target: [u8; 32]) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        Self {
            version: 1,
            prev_block_id: [0; 32].to_vec(),
            merkle_root: [0; 32].to_vec(),
            timestamp,
            target: initial_target.to_vec(),
            nonce: [0; 32].to_vec(),
            index: 0,
        }
    }

    pub fn from_prev_block_id(
        prev_block_id: [u8; 32],
        prev_block_index: u64,
        target: [u8; 32],
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let index = prev_block_index + 1;
        let nonce = [0; 32];
        Self {
            version: 1,
            prev_block_id: prev_block_id.to_vec(),
            merkle_root: [0; 32].to_vec(),
            timestamp,
            target: target.to_vec(),
            nonce: nonce.to_vec(),
            index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_u8_vec_and_from_u8_vec() {
        let bh1 = BlockHeader::new(1, vec![0; 32], vec![0; 32], 0, vec![0; 32], vec![0; 32], 0);
        let buf = bh1.to_u8_vec();
        let bh2 = BlockHeader::from_u8_vec(buf).unwrap();
        assert_eq!(bh1.version, bh2.version);
        assert_eq!(bh1.prev_block_id, bh2.prev_block_id);
        assert_eq!(bh1.merkle_root, bh2.merkle_root);
        assert_eq!(bh1.timestamp, bh2.timestamp);
        assert_eq!(bh1.target, bh2.target);
        assert_eq!(bh1.nonce, bh2.nonce);
        assert_eq!(bh1.index, bh2.index);
    }

    #[test]
    fn test_to_buffer() {
        let bh1 = BlockHeader::new(1, vec![0; 32], vec![0; 32], 0, vec![0; 32], vec![0; 32], 0);
        let buf = bh1.to_u8_vec();
        let bh2 = BlockHeader::from_u8_vec(buf).unwrap();
        assert_eq!(bh1.version, bh2.version);
        assert_eq!(bh1.prev_block_id, bh2.prev_block_id);
        assert_eq!(bh1.merkle_root, bh2.merkle_root);
        assert_eq!(bh1.timestamp, bh2.timestamp);
        assert_eq!(bh1.target, bh2.target);
        assert_eq!(bh1.nonce, bh2.nonce);
        assert_eq!(bh1.index, bh2.index);
    }

    #[test]
    fn test_is_valid() {
        let bh1 = BlockHeader::new(1, vec![0; 32], vec![0; 32], 0, vec![0; 32], vec![0; 32], 0);
        assert!(bh1.is_valid());
    }

    #[test]
    fn test_is_genesis() {
        let bh1 = BlockHeader::new(1, vec![0; 32], vec![0; 32], 0, vec![0; 32], vec![0; 32], 0);
        assert!(bh1.is_genesis());
    }
}
