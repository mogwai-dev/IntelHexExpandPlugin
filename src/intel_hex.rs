use std::io::{self, BufRead, Write};

const DEFAULT_WIDTH: usize = 16;

fn bytes_from_hex(line: &str) -> Result<Vec<u8>, ()> {
    let (pairs, remainder) = line.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(());
    }

    pairs
        .iter()
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|_| ())?;
            u8::from_str_radix(pair, 16).map_err(|_| ())
        })
        .collect()
}

fn write_data_lines<W: Write>(out: &mut W, base: u32, address: u16, data: &[u8]) -> io::Result<()> {
    for (offset, chunk) in data.chunks(DEFAULT_WIDTH).enumerate() {
        let address = base + u32::from(address) + (offset * DEFAULT_WIDTH) as u32;
        write!(out, "{address:08X}:")?;
        for byte in chunk {
            write!(out, " {byte:02X}")?;
        }
        write!(out, "\r\n")?;
    }
    Ok(())
}

pub fn expand<R: BufRead, W: Write>(input: R, mut output: W) -> io::Result<()> {
    let mut base = 0_u32;

    for (line_index, raw_line) in input.lines().enumerate() {
        let raw_line = raw_line?;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            writeln!(output, "{line}")?;
            continue;
        }

        let bytes = match bytes_from_hex(&line[1..]) {
            Ok(bytes) if bytes.len() >= 5 => bytes,
            _ => {
                writeln!(output, "; invalid hex line {}", line_index + 1)?;
                continue;
            }
        };

        let byte_count = usize::from(bytes[0]);
        if bytes.len() != 4 + byte_count + 1 {
            writeln!(output, "; invalid hex length at line {}", line_index + 1)?;
            continue;
        }

        let address = u16::from_be_bytes([bytes[1], bytes[2]]);
        let record_type = bytes[3];
        let data = &bytes[4..4 + byte_count];

        if bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)) != 0 {
            writeln!(output, "; checksum mismatch at line {}", line_index + 1)?;
        }

        match record_type {
            0x00 => write_data_lines(&mut output, base, address, data)?,
            0x01 => break,
            0x02 if data.len() >= 2 => {
                base = u32::from(u16::from_be_bytes([data[0], data[1]])) << 4;
            }
            0x04 if data.len() >= 2 => {
                base = u32::from(u16::from_be_bytes([data[0], data[1]])) << 16;
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::expand;
    use std::io::Cursor;

    fn expand_text(input: &str) -> String {
        let mut output = Vec::new();
        expand(Cursor::new(input), &mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn expands_data_records_in_sixteen_byte_rows() {
        let input = ":10010000214601360121470136007EFE09D2190140\n:00000001FF\n";

        assert_eq!(
            expand_text(input),
            "00000100: 21 46 01 36 01 21 47 01 36 00 7E FE 09 D2 19 01\r\n"
        );
    }

    #[test]
    fn applies_extended_linear_and_segment_addresses() {
        let input = concat!(
            ":020000040001F9\n",
            ":0400100001020304E2\n",
            ":020000021234B6\n",
            ":02001000AABB89\n",
            ":00000001FF\n",
        );

        assert_eq!(
            expand_text(input),
            concat!("00010010: 01 02 03 04\r\n", "00012350: AA BB\r\n",)
        );
    }

    #[test]
    fn reports_malformed_records_and_preserves_plain_text() {
        let input = "header\n:123\n:0200000001FC\n:0100000001FF\n";

        assert_eq!(
            expand_text(input),
            concat!(
                "header\n",
                "; invalid hex line 2\n",
                "; invalid hex length at line 3\n",
                "; checksum mismatch at line 4\n",
                "00000000: 01\r\n",
            )
        );
    }

    #[test]
    fn stops_at_end_of_file_record() {
        let input = ":00000001FF\n:0100000001FE\n";

        assert_eq!(expand_text(input), "");
    }

    #[test]
    fn propagates_input_errors() {
        let input = Cursor::new(vec![b':', 0xFF, b'\n']);

        assert!(expand(input, Vec::new()).is_err());
    }
}
