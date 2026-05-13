use crate::error::{QrdError, Result};
use std::collections::HashMap;

/// Encoding identifiers mandated by Phase 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EncodingId {
    Plain = 0x00,
    Rle = 0x01,
    BitPacked = 0x02,
    DeltaBinary = 0x03,
    DeltaByteArray = 0x04,
    ByteStreamSplit = 0x05,
    DictRle = 0x06,
}

impl EncodingId {
    pub fn from_u8(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Plain),
            0x01 => Ok(Self::Rle),
            0x02 => Ok(Self::BitPacked),
            0x03 => Ok(Self::DeltaBinary),
            0x04 => Ok(Self::DeltaByteArray),
            0x05 => Ok(Self::ByteStreamSplit),
            0x06 => Ok(Self::DictRle),
            other => Err(QrdError::UnsupportedEncoding(other)),
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Encodes values using the selected scaffold encoding.
pub fn encode(values: &[u8], encoding: EncodingId) -> Result<Vec<u8>> {
    match encoding {
        EncodingId::Plain => Ok(values.to_vec()),
        EncodingId::Rle => encode_rle(values),
        EncodingId::BitPacked => encode_bit_packed(values),
        EncodingId::DeltaBinary => encode_delta_binary(values),
        EncodingId::DeltaByteArray => encode_delta_byte_array(values),
        EncodingId::ByteStreamSplit => encode_byte_stream_split(values),
        EncodingId::DictRle => encode_dict_rle(values),
    }
}

/// Decodes values using the selected scaffold encoding.
pub fn decode(values: &[u8], encoding: EncodingId) -> Result<Vec<u8>> {
    match encoding {
        EncodingId::Plain => Ok(values.to_vec()),
        EncodingId::Rle => decode_rle(values),
        EncodingId::BitPacked => decode_bit_packed(values),
        EncodingId::DeltaBinary => decode_delta_binary(values),
        EncodingId::DeltaByteArray => decode_delta_byte_array(values),
        EncodingId::ByteStreamSplit => decode_byte_stream_split(values),
        EncodingId::DictRle => decode_dict_rle(values),
    }
}

fn encode_rle(values: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut cursor = 0usize;
    while cursor < values.len() {
        let value = values[cursor];
        let mut run_length = 1usize;
        while cursor + run_length < values.len()
            && values[cursor + run_length] == value
            && run_length < u16::MAX as usize
        {
            run_length += 1;
        }

        let run_length = u16::try_from(run_length)
            .map_err(|_| QrdError::InvalidSchema("rle run length overflow".into()))?;
        output.extend_from_slice(&run_length.to_le_bytes());
        output.push(value);
        cursor += usize::from(run_length);
    }
    Ok(output)
}

fn decode_rle(values: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut cursor = 0usize;
    while cursor < values.len() {
        let length_bytes = values.get(cursor..cursor + 2).ok_or(QrdError::UnexpectedEof)?;
        let run_length = u16::from_le_bytes([length_bytes[0], length_bytes[1]]);
        cursor += 2;

        let value = *values.get(cursor).ok_or(QrdError::UnexpectedEof)?;
        cursor += 1;

        output.resize(output.len() + usize::from(run_length), value);
    }
    Ok(output)
}

fn encode_bit_packed(values: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let len = u32::try_from(values.len())
        .map_err(|_| QrdError::InvalidSchema("payload too large".into()))?;
    let bit_width = if values.is_empty() {
        0u8
    } else {
        values
            .iter()
            .copied()
            .map(|value| {
                let width = 8 - value.leading_zeros() as u8;
                if width == 0 { 1 } else { width }
            })
            .max()
            .unwrap_or(1)
    };

    output.extend_from_slice(&len.to_le_bytes());
    output.push(bit_width);

    if values.is_empty() {
        return Ok(output);
    }

    let total_bits = usize::from(bit_width) * values.len();
    let mut bit_buffer = Vec::with_capacity(total_bits.div_ceil(8));
    let mut current_byte = 0u8;
    let mut bits_in_current = 0u8;

    for value in values {
        let mut remaining = *value;
        for _ in 0..bit_width {
            let bit = remaining & 1;
            current_byte |= bit << bits_in_current;
            bits_in_current += 1;
            remaining >>= 1;
            if bits_in_current == 8 {
                bit_buffer.push(current_byte);
                current_byte = 0;
                bits_in_current = 0;
            }
        }
    }

    if bits_in_current > 0 {
        bit_buffer.push(current_byte);
    }

    output.extend_from_slice(&bit_buffer);
    Ok(output)
}

fn decode_bit_packed(values: &[u8]) -> Result<Vec<u8>> {
    let header = values.get(0..5).ok_or(QrdError::UnexpectedEof)?;
    let original_len = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let bit_width = header[4] as usize;

    if original_len == 0 {
        return Ok(Vec::new());
    }

    if bit_width == 0 || bit_width > 8 {
        return Err(QrdError::InvalidSchema("invalid bit width".into()));
    }

    let payload = values.get(5..).ok_or(QrdError::UnexpectedEof)?;
    let total_bits = original_len.checked_mul(bit_width).ok_or_else(|| QrdError::InvalidSchema("bit stream split length overflow".into()))?;
    let expected_bytes = total_bits.div_ceil(8);
    if payload.len() != expected_bytes {
        return Err(QrdError::UnexpectedEof);
    }

    let mut output = Vec::with_capacity(original_len);
    let mut bit_cursor = 0;
    for _ in 0..original_len {
        let mut value = 0u8;
        for bit_index in 0..bit_width {
            let byte_index = bit_cursor / 8;
            let bit_offset = bit_cursor % 8;
            let bit = (payload[byte_index] >> bit_offset) & 1;
            value |= bit << bit_index;
            bit_cursor += 1;
        }
        output.push(value);
    }

    Ok(output)
}

fn encode_delta_binary(values: &[u8]) -> Result<Vec<u8>> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity(values.len());
    output.push(values[0]);
    for index in 1..values.len() {
        output.push(values[index].wrapping_sub(values[index - 1]));
    }
    Ok(output)
}

fn decode_delta_binary(values: &[u8]) -> Result<Vec<u8>> {
    if values.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity(values.len());
    output.push(values[0]);
    for index in 1..values.len() {
        let next = output[index - 1].wrapping_add(values[index]);
        output.push(next);
    }
    Ok(output)
}

fn encode_delta_byte_array(values: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(values.len() + 4);
    let len = u32::try_from(values.len())
        .map_err(|_| QrdError::InvalidSchema("payload too large".into()))?;
    output.extend_from_slice(&len.to_le_bytes());

    if values.is_empty() {
        return Ok(output);
    }

    output.push(values[0]);
    for index in 1..values.len() {
        output.push(values[index].wrapping_sub(values[index - 1]));
    }
    Ok(output)
}

fn decode_delta_byte_array(values: &[u8]) -> Result<Vec<u8>> {
    let length_bytes = values.get(0..4).ok_or(QrdError::UnexpectedEof)?;
    let original_len = u32::from_le_bytes([
        length_bytes[0],
        length_bytes[1],
        length_bytes[2],
        length_bytes[3],
    ]) as usize;

    if original_len == 0 {
        return Ok(Vec::new());
    }

    let payload = values.get(4..).ok_or(QrdError::UnexpectedEof)?;
    if payload.is_empty() {
        return Err(QrdError::UnexpectedEof);
    }

    let mut output = Vec::with_capacity(original_len);
    output.push(payload[0]);
    for index in 1..payload.len() {
        output.push(output[index - 1].wrapping_add(payload[index]));
    }

    if output.len() != original_len {
        return Err(QrdError::UnexpectedEof);
    }
    Ok(output)
}

fn encode_byte_stream_split(values: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(4 + values.len().saturating_mul(8));
    let len = u32::try_from(values.len())
        .map_err(|_| QrdError::InvalidSchema("payload too large".into()))?;
    output.extend_from_slice(&len.to_le_bytes());

    for bit_plane in 0..8 {
        for value in values {
            output.push((value >> bit_plane) & 1);
        }
    }
    Ok(output)
}

fn decode_byte_stream_split(values: &[u8]) -> Result<Vec<u8>> {
    let length_bytes = values.get(0..4).ok_or(QrdError::UnexpectedEof)?;
    let original_len = u32::from_le_bytes([
        length_bytes[0],
        length_bytes[1],
        length_bytes[2],
        length_bytes[3],
    ]) as usize;

    let payload = values.get(4..).ok_or(QrdError::UnexpectedEof)?;
    let expected_len = original_len.checked_mul(8).ok_or_else(|| {
        QrdError::InvalidSchema("bit stream split length overflow".into())
    })?;
    if payload.len() != expected_len {
        return Err(QrdError::UnexpectedEof);
    }

    let mut output = vec![0u8; original_len];
    let mut cursor = 0usize;
    for bit_plane in 0..8 {
        for output_byte in output.iter_mut().take(original_len) {
            let bit = payload[cursor] & 1;
            *output_byte |= bit << bit_plane;
            cursor += 1;
        }
    }
    Ok(output)
}

fn encode_dict_rle(values: &[u8]) -> Result<Vec<u8>> {
    let mut dictionary = Vec::new();
    let mut index_map = HashMap::new();

    for value in values {
        index_map.entry(*value).or_insert_with(|| {
            let next_index = dictionary.len() as u8;
            dictionary.push(*value);
            next_index
        });
    }

    let mut output = Vec::new();
    output.push(dictionary.len() as u8);
    output.extend_from_slice(&dictionary);
    let len = u32::try_from(values.len())
        .map_err(|_| QrdError::InvalidSchema("payload too large".into()))?;
    output.extend_from_slice(&len.to_le_bytes());

    for value in values {
        let index = index_map
            .get(value)
            .copied()
            .ok_or_else(|| QrdError::InvalidSchema("dictionary encoding failure".into()))?;
        output.push(index);
    }
    Ok(output)
}

fn decode_dict_rle(values: &[u8]) -> Result<Vec<u8>> {
    let dictionary_len = *values.first().ok_or(QrdError::UnexpectedEof)? as usize;
    let dictionary = values.get(1..1 + dictionary_len).ok_or(QrdError::UnexpectedEof)?;
    let length_offset = 1 + dictionary_len;
    let length_bytes = values.get(length_offset..length_offset + 4).ok_or(QrdError::UnexpectedEof)?;
    let original_len = u32::from_le_bytes([
        length_bytes[0],
        length_bytes[1],
        length_bytes[2],
        length_bytes[3],
    ]) as usize;

    let indices = values.get(length_offset + 4..).ok_or(QrdError::UnexpectedEof)?;
    if indices.len() != original_len {
        return Err(QrdError::UnexpectedEof);
    }

    let mut output = Vec::with_capacity(original_len);
    for index in indices {
        let value = *dictionary.get(*index as usize).ok_or(QrdError::UnexpectedEof)?;
        output.push(value);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(values: &[u8], encoding: EncodingId) {
        let encoded = encode(values, encoding).expect("encoding should succeed");
        let decoded = decode(&encoded, encoding).expect("decoding should succeed");
        assert_eq!(decoded, values);
    }

    #[test]
    fn all_encodings_roundtrip() {
        let sample = [0u8, 0, 1, 1, 1, 2, 3, 3, 4, 5, 8, 13, 21];
        roundtrip(&sample, EncodingId::Plain);
        roundtrip(&sample, EncodingId::Rle);
        roundtrip(&sample, EncodingId::BitPacked);
        roundtrip(&sample, EncodingId::DeltaBinary);
        roundtrip(&sample, EncodingId::DeltaByteArray);
        roundtrip(&sample, EncodingId::ByteStreamSplit);
        roundtrip(&sample, EncodingId::DictRle);
    }

    #[test]
    fn unknown_encoding_id_is_rejected() {
        let error = EncodingId::from_u8(0xFF).expect_err("invalid encoding id must fail");
        assert!(matches!(error, QrdError::UnsupportedEncoding(0xFF)));
    }
}