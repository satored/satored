use crate::error::EbxError;
use crate::numbers::u256;
use byteorder::{BigEndian, ReadBytesExt};
use std::{io::Cursor, vec};

pub struct BufReader {
    buf: Cursor<Vec<u8>>,
}

impl BufReader {
    pub fn new(buf: Vec<u8>) -> BufReader {
        BufReader {
            buf: Cursor::new(buf),
        }
    }

    pub fn eof(&self) -> bool {
        self.buf.position() as usize >= self.buf.get_ref().len()
    }

    pub fn remainder_len(&self) -> usize {
        self.buf.get_ref().len() - self.buf.position() as usize
    }

    pub fn read(&mut self, len: usize) -> Result<Vec<u8>, EbxError> {
        let pos = self.buf.position() as usize;
        if pos + len > self.buf.get_ref().len() {
            return Err(EbxError::NotEnoughDataError { source: None });
        }
        let buf = self.buf.get_ref()[pos..pos + len].to_vec();
        self.buf.set_position((pos + len) as u64);
        Ok(buf)
    }

    pub fn read_remainder(&mut self) -> Vec<u8> {
        let pos = self.buf.position() as usize;
        let buf = self.buf.get_ref()[pos..].to_vec();
        self.buf.set_position(self.buf.get_ref().len() as u64);
        buf
    }

    pub fn read_u8(&mut self) -> Result<u8, EbxError> {
        self.buf
            .read_u8()
            .map_err(|_| EbxError::NotEnoughDataError { source: None })
    }

    pub fn read_u16_be(&mut self) -> Result<u16, EbxError> {
        self.buf
            .read_u16::<BigEndian>()
            .map_err(|_| EbxError::NotEnoughDataError { source: None })
    }

    pub fn read_u32_be(&mut self) -> Result<u32, EbxError> {
        self.buf
            .read_u32::<BigEndian>()
            .map_err(|_| EbxError::NotEnoughDataError { source: None })
    }

    pub fn read_u64_be(&mut self) -> Result<u64, EbxError> {
        self.buf
            .read_u64::<BigEndian>()
            .map_err(|_| EbxError::NotEnoughDataError { source: None })
    }

    pub fn read_u128_be(&mut self) -> Result<u128, EbxError> {
        self.buf
            .read_u128::<BigEndian>()
            .map_err(|_| EbxError::NotEnoughDataError { source: None })
    }

    pub fn read_u256_be(&mut self) -> Result<u256, EbxError> {
        let val1 = self.read_u64_be()?;
        let val2 = self.read_u64_be()?;
        let val3 = self.read_u64_be()?;
        let val4 = self.read_u64_be()?;

        // from_digits is little endian, so we need to reverse the order
        let val = u256::from_digits([val4, val3, val2, val1]);

        Ok(val)
    }

    pub fn read_var_int_buf(&mut self) -> Result<Vec<u8>, EbxError> {
        let first = self.read_u8().map_err(|e| EbxError::NotEnoughDataError {
            source: Some(Box::new(e)),
        })?;
        match first {
            0xfd => {
                let mut buf = vec![first];
                buf.extend_from_slice(&self.read(2).map_err(|e| EbxError::NotEnoughDataError {
                    source: Some(Box::new(e)),
                })?);
                if Cursor::new(&buf[1..]).read_u16::<BigEndian>().unwrap() < 0xfd {
                    return Err(EbxError::NonMinimalEncodingError { source: None });
                }
                Ok(buf)
            }
            0xfe => {
                let mut buf = vec![first];
                buf.extend_from_slice(&self.read(4).map_err(|e| EbxError::NotEnoughDataError {
                    source: Some(Box::new(e)),
                })?);

                if Cursor::new(&buf[1..]).read_u32::<BigEndian>().unwrap() < 0x10000 {
                    return Err(EbxError::NonMinimalEncodingError { source: None });
                }
                Ok(buf)
            }
            0xff => {
                let mut buf = vec![first];
                buf.extend_from_slice(&self.read(8).map_err(|e| EbxError::NotEnoughDataError {
                    source: Some(Box::new(e)),
                })?);
                if Cursor::new(&buf[1..]).read_u64::<BigEndian>().unwrap() < 0x100000000 {
                    return Err(EbxError::NonMinimalEncodingError { source: None });
                }
                Ok(buf)
            }
            _ => Ok(vec![first]),
        }
    }

    pub fn read_var_int(&mut self) -> Result<u64, EbxError> {
        let buf = self.read_var_int_buf()?;
        let first = buf[0];
        match first {
            0xfd => Ok(Cursor::new(&buf[1..]).read_u16::<BigEndian>().unwrap() as u64),
            0xfe => Ok(Cursor::new(&buf[1..]).read_u32::<BigEndian>().unwrap() as u64),
            0xff => Ok(Cursor::new(&buf[1..]).read_u64::<BigEndian>().unwrap()),
            _ => Ok(first as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::EbxBuf;
    use byteorder::{BigEndian, WriteBytesExt};
    use serde::Deserialize;
    use std::fs;

    #[test]
    fn test_read() {
        let mut reader = BufReader::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(reader.read(3).unwrap(), vec![1, 2, 3]);
        assert_eq!(reader.read(2).unwrap(), vec![4, 5]);
    }

    #[test]
    fn test_read_u8() {
        let mut reader = BufReader::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(reader.read_u8().unwrap(), 1);
        assert_eq!(reader.read_u8().unwrap(), 2);
    }

    #[test]
    fn test_read_u16_be() {
        let mut buffer_reader = BufReader::new(vec![0x01, 0x23]);
        assert_eq!(buffer_reader.read_u16_be().unwrap(), 0x0123);
    }

    #[test]
    fn test_read_u32_be() {
        let mut data = vec![];
        data.write_u32::<BigEndian>(1234567890).unwrap();
        data.write_u32::<BigEndian>(987654321).unwrap();

        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_u32_be().unwrap(), 1234567890);
        assert_eq!(reader.read_u32_be().unwrap(), 987654321);
    }

    #[test]
    fn test_read_u64_be_big_int() {
        let mut data = vec![];
        data.write_u64::<BigEndian>(12345678901234567890).unwrap();
        data.write_u64::<BigEndian>(9876543210987654321).unwrap();

        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_u64_be().unwrap(), 12345678901234567890);
        assert_eq!(reader.read_u64_be().unwrap(), 9876543210987654321);
    }

    #[test]
    fn test_read_var_int_buf() {
        let data = vec![0xfd, 0x01, 0x00];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int_buf().unwrap(), vec![0xfd, 0x01, 0x00]);

        let data = vec![0xfe, 0x01, 0x00, 0x00, 0x00];
        let mut reader = BufReader::new(data);
        assert_eq!(
            reader.read_var_int_buf().unwrap(),
            vec![0xfe, 0x01, 0x00, 0x00, 0x00]
        );

        let data = vec![0xff, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut reader = BufReader::new(data);
        assert_eq!(
            reader.read_var_int_buf().unwrap(),
            vec![0xff, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );

        let data = vec![0x01];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int_buf().unwrap(), vec![0x01]);
    }

    #[test]
    fn test_read_var_int() {
        let data = vec![0xfd, 0x10, 0x01];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int().unwrap(), 0x1000 + 1);

        let data = vec![0xfe, 0x10, 0x00, 0x00, 0x01];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int().unwrap(), 0x10000000 + 1);

        let data = vec![0xff, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int().unwrap(), 0x1000000000000000 + 1);

        let data = vec![0x01];
        let mut reader = BufReader::new(data);
        assert_eq!(reader.read_var_int().unwrap(), 1);
    }

    // standard test vectors

    #[derive(Deserialize)]
    struct TestVectorIsoBufReader {
        read: TestVectorReadIsoBuf,
        read_u8: TestVectorReadErrors,
        read_u16_be: TestVectorReadErrors,
        read_u32_be: TestVectorReadErrors,
        read_u64_be: TestVectorReadErrors,
        read_var_int_buf: TestVectorReadErrors,
        read_var_int: TestVectorReadErrors,
    }

    #[derive(Deserialize)]
    struct TestVectorReadIsoBuf {
        errors: Vec<TestVectorReadIsoBufError>,
    }

    #[derive(Deserialize)]
    struct TestVectorReadIsoBufError {
        hex: String,
        len: usize,
        error: String,
    }

    #[derive(Deserialize)]
    struct TestVectorReadErrors {
        errors: Vec<TestVectorReadError>,
    }

    #[derive(Deserialize)]
    struct TestVectorReadError {
        hex: String,
        error: String,
    }

    #[test]
    fn test_vectors_read() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read(test_vector.len);
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }

    #[test]
    fn test_vectors_read_u8() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_u8.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_u8();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert_eq!(e.to_string(), test_vector.error),
            }
        }
    }

    #[test]
    fn test_vectors_read_u16_be() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_u16_be.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_u16_be();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }

    #[test]
    fn test_vectors_read_u32_be() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_u32_be.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_u32_be();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }

    #[test]
    fn test_vectors_read_u64_be() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_u64_be.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_u64_be();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }

    #[test]
    fn test_vectors_read_var_int_buf() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_var_int_buf.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_var_int_buf();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }

    #[test]
    fn test_vectors_read_var_int() {
        let data =
            fs::read_to_string("./test_vectors/buf_reader.json").expect("Unable to read file");
        let test_vectors: TestVectorIsoBufReader =
            serde_json::from_str(&data).expect("Unable to parse JSON");
        for test_vector in test_vectors.read_var_int.errors {
            let buf = Vec::<u8>::from_strict_hex(&test_vector.hex).expect("Failed to decode hex");
            let mut reader = BufReader::new(buf);
            let result = reader.read_var_int();
            match result {
                Ok(_) => panic!("Expected an error, but got Ok(_)"),
                Err(e) => assert!(e.to_string().starts_with(&test_vector.error)),
            }
        }
    }
}
