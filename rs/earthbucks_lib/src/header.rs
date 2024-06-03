use crate::buf_reader::BufReader;
use crate::buf_writer::BufWriter;
use crate::ebx_buf::IsoBuf;
use crate::ebx_error::EbxError;
use crate::hash::{blake3_hash, double_blake3_hash};
use num_bigint::BigUint;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Header {
    pub version: u32,
    pub prev_block_id: [u8; 32],
    pub timestamp: u64, // seconds
    pub block_num: u64,
    pub merkle_root: [u8; 32],
    pub target: [u8; 32],
    pub nonce: [u8; 32],
    pub work_ser_algo: u32,
    pub work_ser_hash: [u8; 32],
    pub work_par_algo: u32,
    pub work_par_hash: [u8; 32],
}

impl Header {
    pub const BLOCKS_PER_TARGET_ADJ_PERIOD: u64 = 2016; // exactly two weeks if block interval is 10 minutes
    pub const BLOCK_INTERVAL: u64 = 600; // 600 seconds = 10 minutes
    pub const SIZE: usize = 220;
    pub const INITIAL_TARGET: [u8; 32] = [0xff; 32];

    pub fn to_iso_buf(&self) -> [u8; Header::SIZE] {
        let mut bw = BufWriter::new();
        bw.write_u32_be(self.version);
        bw.write(self.prev_block_id.to_vec());
        bw.write(self.merkle_root.to_vec());
        bw.write_u64_be(self.timestamp);
        bw.write_u64_be(self.block_num);
        bw.write(self.target.to_vec());
        bw.write(self.nonce.to_vec());
        bw.write_u32_be(self.work_ser_algo);
        bw.write(self.work_ser_hash.to_vec());
        bw.write_u32_be(self.work_par_algo);
        bw.write(self.work_par_hash.to_vec());
        bw.to_buf().try_into().unwrap()
    }

    pub fn from_iso_buf(buf: [u8; Header::SIZE]) -> Result<Header, EbxError> {
        let mut br = BufReader::new(buf.to_vec());
        let version = br.read_u32_be()?;
        let prev_block_id: [u8; 32] = br.read(32)?.try_into().unwrap();
        let merkle_root: [u8; 32] = br.read(32)?.try_into().unwrap();
        let timestamp = br.read_u64_be()?;
        let block_num = br.read_u64_be()?;
        let target: [u8; 32] = br.read(32)?.try_into().unwrap();
        let nonce: [u8; 32] = br.read(32)?.try_into().unwrap();
        let work_ser_algo = br.read_u32_be()?;
        let work_ser_hash: [u8; 32] = br.read(32)?.try_into().unwrap();
        let work_par_algo = br.read_u32_be()?;
        let work_par_hash: [u8; 32] = br.read(32)?.try_into().unwrap();
        Ok(Self {
            version,
            prev_block_id,
            merkle_root,
            timestamp,
            block_num,
            target,
            nonce,
            work_ser_algo,
            work_ser_hash,
            work_par_algo,
            work_par_hash,
        })
    }

    pub fn from_iso_buf_reader(br: &mut BufReader) -> Result<Header, EbxError> {
        if br.remainder_len() < Header::SIZE {
            panic!("Invalid block header size");
        }
        let version = br.read_u32_be()?;
        let prev_block_id: [u8; 32] = br.read(32)?.try_into().unwrap();
        let merkle_root: [u8; 32] = br.read(32)?.try_into().unwrap();
        let timestamp = br.read_u64_be()?;
        let block_num = br.read_u64_be()?;
        let target: [u8; 32] = br.read(32)?.try_into().unwrap();
        let nonce: [u8; 32] = br.read(32)?.try_into().unwrap();
        let work_ser_algo: u32 = br.read_u32_be()?;
        let work_ser_hash: [u8; 32] = br.read(32)?.try_into().unwrap();
        let work_par_algo: u32 = br.read_u32_be()?;
        let work_par_hash: [u8; 32] = br.read(32)?.try_into().unwrap();
        Ok(Self {
            version,
            prev_block_id,
            merkle_root,
            timestamp,
            block_num,
            target,
            nonce,
            work_ser_algo,
            work_ser_hash,
            work_par_algo,
            work_par_hash,
        })
    }

    pub fn to_iso_buf_writer(&self) -> BufWriter {
        let mut bw = BufWriter::new();
        bw.write_u32_be(self.version);
        bw.write(self.prev_block_id.to_vec());
        bw.write(self.merkle_root.to_vec());
        bw.write_u64_be(self.timestamp);
        bw.write_u64_be(self.block_num);
        bw.write(self.target.to_vec());
        bw.write(self.nonce.to_vec());
        bw.write_u32_be(self.work_ser_algo);
        bw.write(self.work_ser_hash.to_vec());
        bw.write_u32_be(self.work_par_algo);
        bw.write(self.work_par_hash.to_vec());
        bw
    }

    pub fn to_iso_hex(&self) -> String {
        self.to_iso_buf().to_strict_hex()
    }

    pub fn from_iso_hex(hex: &str) -> Result<Header, EbxError> {
        let buf: [u8; Header::SIZE] = Vec::<u8>::from_strict_hex(hex)
            .map_err(|_| EbxError::InvalidHexError { source: None })?
            .try_into()
            .map_err(|_| EbxError::InvalidHexError { source: None })?;
        Header::from_iso_buf(buf)
    }

    pub fn to_iso_str(&self) -> String {
        self.to_iso_hex()
    }

    pub fn from_iso_str(hex: &str) -> Result<Header, EbxError> {
        Header::from_iso_hex(hex)
    }

    pub fn is_valid_target(&self, lch: &[Header]) -> bool {
        let new_target_res = Header::new_target_from_lch(lch, self.timestamp);
        if new_target_res.is_err() {
            return false;
        }
        let new_target = new_target_res.unwrap();
        let target = BigUint::from_bytes_be(&self.target);
        let new_target_num = BigUint::from_bytes_be(&new_target);
        target == new_target_num
    }

    pub fn is_valid_pow(&self) -> bool {
        let id = self.id();
        let target = BigUint::from_bytes_be(&self.target);
        let id_num = BigUint::from_bytes_be(&id);
        id_num < target
    }

    pub fn is_valid_version(version: u32) -> bool {
        version == 1
    }

    pub fn is_valid_in_isolation(&self) -> bool {
        let len = self.to_iso_buf().len();
        if len != Header::SIZE {
            return false;
        }
        Header::is_valid_version(self.version)
    }

    pub fn is_valid_at_timestamp(&self, timestamp: u64) -> bool {
        if self.timestamp > timestamp {
            return false;
        }
        true
    }

    pub fn is_valid_in_lch(&self, lch: &[Header]) -> bool {
        if !self.is_valid_in_isolation() {
            return false;
        }
        if self.block_num == 0 {
            return self.is_genesis();
        }
        if lch.is_empty() {
            return false;
        }
        if self.block_num != lch.len() as u64 {
            return false;
        }
        if self.prev_block_id != lch.last().unwrap().id() {
            return false;
        }
        if self.timestamp <= lch.last().unwrap().timestamp {
            return false;
        }
        if !self.is_valid_target(lch) {
            return false;
        }
        if !self.is_valid_pow() {
            return false;
        }
        true
    }

    pub fn is_valid_at(&self, lch: &[Header], timestamp: u64) -> bool {
        self.is_valid_in_lch(lch) && self.is_valid_at_timestamp(timestamp)
    }

    pub fn is_valid_now(&self, lch: &[Header]) -> bool {
        let now = Header::get_new_timestamp();
        self.is_valid_at(lch, now)
    }

    pub fn is_genesis(&self) -> bool {
        self.block_num == 0 && self.prev_block_id.iter().all(|&x| x == 0)
    }

    pub fn from_genesis(now: u64) -> Self {
        let initial_target = Header::INITIAL_TARGET;
        let timestamp = now;
        Self {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp,
            block_num: 0,
            target: initial_target,
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        blake3_hash(&self.to_iso_buf())
    }

    pub fn id(&self) -> [u8; 32] {
        double_blake3_hash(&self.to_iso_buf())
    }

    pub fn get_new_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }

    pub fn from_lch(lch: &[Header], new_timestamp: u64) -> Result<Self, String> {
        if lch.is_empty() {
            return Ok(Header::from_genesis(new_timestamp));
        }
        let new_target_res = Header::new_target_from_lch(lch, new_timestamp);
        if new_target_res.is_err() {
            return Err("new target error".to_string());
        }
        let prev_block = lch.last().unwrap();
        let new_target = new_target_res.unwrap();
        let prev_block_id = prev_block.id();
        let block_num = lch.len() as u64;
        let timestamp = new_timestamp;
        let nonce = [0u8; 32];
        let work_ser_algo = prev_block.work_ser_algo;
        let work_ser_hash = [0u8; 32];
        let work_par_algo = prev_block.work_par_algo;
        let work_par_hash = [0u8; 32];
        Ok(Self {
            version: 1,
            prev_block_id,
            merkle_root: [0u8; 32],
            timestamp,
            block_num,
            target: new_target,
            nonce,
            work_ser_algo,
            work_ser_hash,
            work_par_algo,
            work_par_hash,
        })
    }

    pub fn new_target_from_lch(lch: &[Header], new_timestamp: u64) -> Result<[u8; 32], String> {
        // get slice of max length BLOCKS_PER_TARGET_ADJ
        let adjh: Vec<Header> = if lch.len() > Header::BLOCKS_PER_TARGET_ADJ_PERIOD as usize {
            lch[lch.len() - Header::BLOCKS_PER_TARGET_ADJ_PERIOD as usize..].to_vec()
        } else {
            lch.to_vec().clone()
        };

        // convert all targets into big numbers
        let len = adjh.len();
        if len == 0 {
            return Ok(Header::INITIAL_TARGET);
        }
        let first_header = adjh[0].clone();
        // let last_header = adjh[len - 1].clone();
        let mut targets: Vec<BigUint> = Vec::new();
        for header in adjh {
            let target = BigUint::from_bytes_be(&header.target);
            targets.push(target);
        }
        let target_sum: BigUint = targets.iter().sum();
        if new_timestamp <= first_header.timestamp {
            // error
            return Err("timestamps must be increasing".to_string());
        }
        let real_time_diff: BigUint =
            BigUint::from(new_timestamp) - BigUint::from(first_header.timestamp);
        let new_target = Header::new_target_from_old_targets(target_sum, real_time_diff, len);

        let mut new_target_bytes = new_target.to_bytes_be();
        while new_target_bytes.len() < 32 {
            new_target_bytes.insert(0, 0);
        }
        if new_target_bytes.len() > 32 {
            new_target_bytes = Header::INITIAL_TARGET.to_vec();
        }
        Ok(new_target_bytes.try_into().unwrap())
    }

    pub fn new_target_from_old_targets(
        target_sum: BigUint,
        real_time_diff: BigUint,
        len_usize: usize,
    ) -> BigUint {
        // - target_sum is sum of all targets in the adjustment period
        // - real_time_diff is the time difference between the first block in
        //   the adjustment period and now (the new block)
        // new target = average target * real time diff / intended time diff
        // let new_target = (target_sum / len) * real_time_diff / intended_time_diff;
        // let new_target = (target_sum * real_time_diff) / intended_time_diff / len;
        // let new_target = (target_sum * real_time_diff) / len / intended_time_diff;
        // let new_target = (target_sum * real_time_diff) / (len * intended_time_diff);
        // the fewest divisions is the most accurate in integer arithmetic...
        let intended_time_diff = BigUint::from(len_usize as u64 * Header::BLOCK_INTERVAL);
        let len = BigUint::from(len_usize);
        (target_sum * real_time_diff) / (len * intended_time_diff)
    }

    pub fn coinbase_amount(block_num: u64) -> u64 {
        // shift every 210,000 blocks
        let shift_by = block_num / 210_000;
        (100 * 100_000_000) >> shift_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_iso_buf_and_from_iso_buf() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        let buf = bh1.to_iso_buf();
        let bh2 = Header::from_iso_buf(buf).unwrap();
        assert_eq!(bh1.version, bh2.version);
        assert_eq!(bh1.prev_block_id, bh2.prev_block_id);
        assert_eq!(bh1.merkle_root, bh2.merkle_root);
        assert_eq!(bh1.timestamp, bh2.timestamp);
        assert_eq!(bh1.target, bh2.target);
        assert_eq!(bh1.nonce, bh2.nonce);
        assert_eq!(bh1.block_num, bh2.block_num);
    }

    #[test]
    fn test_to_buffer() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        let buf = bh1.to_iso_buf();
        let bh2 = Header::from_iso_buf(buf).unwrap();
        assert_eq!(bh1.version, bh2.version);
        assert_eq!(bh1.prev_block_id, bh2.prev_block_id);
        assert_eq!(bh1.merkle_root, bh2.merkle_root);
        assert_eq!(bh1.timestamp, bh2.timestamp);
        assert_eq!(bh1.target, bh2.target);
        assert_eq!(bh1.nonce, bh2.nonce);
        assert_eq!(bh1.block_num, bh2.block_num);
    }

    #[test]
    fn test_is_valid() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        assert!(bh1.is_valid_in_isolation());
    }

    #[test]
    fn test_is_genesis() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        assert!(bh1.is_genesis());
    }

    #[test]
    fn test_hash() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        let hash = bh1.hash();
        let hex = hash.to_strict_hex();
        assert_eq!(
            hex,
            "207308090b4e6af2f1b46b22b849506534536fb39ca5976548f1032e2360ff00"
        );
    }

    #[test]
    fn test_id() {
        let bh1 = Header {
            version: 1,
            prev_block_id: [0; 32],
            merkle_root: [0; 32],
            timestamp: 0,
            block_num: 0,
            target: [0; 32],
            nonce: [0; 32],
            work_ser_algo: 0,
            work_ser_hash: [0; 32],
            work_par_algo: 0,
            work_par_hash: [0; 32],
        };
        let id = bh1.id();
        let hex = id.to_strict_hex();
        assert_eq!(
            hex,
            "24f3f2f083a1accdbc64581b928fbde7f623756c45a17f5730ff7019b424360e"
        );
    }

    #[test]
    fn test_coinbase_amount() {
        assert_eq!(Header::coinbase_amount(0), 10_000_000_000);
        assert_eq!(Header::coinbase_amount(210_000), 5_000_000_000);
        assert_eq!(Header::coinbase_amount(420_000), 2_500_000_000);
        assert_eq!(Header::coinbase_amount(630_000), 1_250_000_000);
        assert_eq!(Header::coinbase_amount(840_000), 625_000_000);
        assert_eq!(Header::coinbase_amount(1_050_000), 312_500_000);
        assert_eq!(Header::coinbase_amount(1_260_000), 156_250_000);

        let mut sum = 0;
        for i in 0..2_000_000 {
            sum += Header::coinbase_amount(i);
        }
        assert_eq!(sum, 4193945312500000);
    }

    #[test]
    fn test_new_target_from_old_targets_1() {
        let target_1_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());

        let target_sum = target_1;
        let real_time_diff: u64 = 600;
        let len: usize = 1;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_1_2() {
        let target_1_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());

        let target_sum = target_1;
        let real_time_diff: u64 = 300;
        let len: usize = 1;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_1_3() {
        let target_1_hex = "8000000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());

        let target_sum = target_1;
        let real_time_diff: u64 = 600;
        let len: usize = 1;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "8000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_1_4() {
        let target_1_hex = "8000000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());

        let target_sum = target_1;
        let real_time_diff: u64 = 300;
        let len: usize = 1;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "4000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_1_5() {
        let target_1_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());

        let target_sum = target_1;
        let real_time_diff: u64 = 1200;
        let len: usize = 1;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "0100000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_2() {
        let target_1_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_sum = target_1 + target_2;
        let real_time_diff: u64 = 600 + 600;
        let len: usize = 2;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_2_2() {
        let target_1_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_sum = target_1 + target_2;
        let real_time_diff: u64 = 600 + 300;
        let len: usize = 2;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex: String = format!("{:0>64}", hex::encode(new_target.to_bytes_be()));
        let expected_hex = "0060000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_2_3() {
        let target_1_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_sum = target_1 + target_2;
        let real_time_diff: u64 = 600 + 1200;
        let len: usize = 2;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex: String = format!("{:0>64}", hex::encode(new_target.to_bytes_be()));
        let expected_hex = "00c0000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_3() {
        let target_1_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_3_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let target_3: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_3_hex).unwrap());
        let target_sum = target_1 + target_2 + target_3;
        let real_time_diff: u64 = 600 + 600 + 600;
        let len: usize = 3;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex = hex::encode(new_target.to_bytes_be());
        let expected_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_3_2() {
        let target_1_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_3_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_3: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_3_hex).unwrap());
        let target_sum = target_1 + target_2 + target_3;
        let real_time_diff: u64 = 600 + 600 + 601;
        let len: usize = 3;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex: String = format!("{:0>64}", hex::encode(new_target.to_bytes_be()));
        let expected_hex = "0080123456789abcdf0123456789abcdf0123456789abcdf0123456789abcdf0";
        assert_eq!(new_target_hex, expected_hex);
    }

    #[test]
    fn test_new_target_from_old_targets_3_3() {
        let target_1_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_1: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_1_hex).unwrap());
        let target_2_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_2: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_2_hex).unwrap());
        let target_3_hex = "0080000000000000000000000000000000000000000000000000000000000000";
        let target_3: BigUint =
            BigUint::from_bytes_be(&Vec::<u8>::from_strict_hex(target_3_hex).unwrap());
        let target_sum = target_1 + target_2 + target_3;
        let real_time_diff: u64 = 600 + 600 + 599;
        let len: usize = 3;
        let new_target =
            Header::new_target_from_old_targets(target_sum, real_time_diff.into(), len);
        let new_target_hex: String = format!("{:0>64}", hex::encode(new_target.to_bytes_be()));
        let expected_hex = "007fedcba987654320fedcba987654320fedcba987654320fedcba987654320f";
        assert_eq!(new_target_hex, expected_hex);
    }
}
