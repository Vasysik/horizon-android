//! A bounded, read-only summary of Android's protobuf tombstone format.
//! Field numbers: https://android.googlesource.com/platform/system/core/+/refs/heads/main/debuggerd/proto/tombstone.proto
//! Deliberately excludes registers, memory dumps, logs, file descriptors and command lines.
#[derive(Clone, Copy)]
enum Value<'a> {
    Number(u64),
    Bytes(&'a [u8]),
}
fn varint(data: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut out = 0;
    for shift in (0..70).step_by(7) {
        let b = *data.get(*cursor)?;
        *cursor += 1;
        if shift == 63 && b > 1 {
            return None;
        }
        out |= u64::from(b & 127) << shift;
        if b & 128 == 0 {
            return Some(out);
        }
    }
    None
}
fn fields(data: &[u8]) -> Vec<(u32, Value<'_>)> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < data.len() && out.len() < 10000 {
        let Some(tag) = varint(data, &mut cursor) else {
            break;
        };
        let field = (tag >> 3) as u32;
        if field == 0 {
            break;
        }
        let value = match tag & 7 {
            0 => varint(data, &mut cursor).map(Value::Number),
            2 => {
                let Some(len) = varint(data, &mut cursor).and_then(|n| usize::try_from(n).ok())
                else {
                    break;
                };
                let Some(end) = cursor.checked_add(len) else {
                    break;
                };
                let Some(bytes) = data.get(cursor..end) else {
                    break;
                };
                cursor = end;
                Some(Value::Bytes(bytes))
            }
            wire @ (1 | 5) => {
                cursor = cursor.saturating_add(if wire == 1 { 8 } else { 4 });
                continue;
            }
            _ => break,
        };
        let Some(value) = value else {
            break;
        };
        out.push((field, value));
    }
    out
}
fn number(values: &[(u32, Value<'_>)], field: u32) -> u64 {
    values
        .iter()
        .find_map(|(id, v)| match v {
            Value::Number(n) if *id == field => Some(*n),
            _ => None,
        })
        .unwrap_or(0)
}
fn bytes<'a>(values: &[(u32, Value<'a>)], field: u32) -> Option<&'a [u8]> {
    values.iter().find_map(|(id, v)| match v {
        Value::Bytes(b) if *id == field => Some(*b),
        _ => None,
    })
}
fn text(values: &[(u32, Value<'_>)], field: u32) -> String {
    bytes(values, field)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("")
        .chars()
        .take(1024)
        .collect()
}
pub fn summarize(data: &[u8]) -> Option<String> {
    use std::fmt::Write;
    let root = fields(data.get(..data.len().min(1024 * 1024))?);
    let signal = fields(bytes(&root, 10)?);
    let name = text(&signal, 2);
    if number(&signal, 1) == 0 || !name.starts_with("SIG") {
        return None;
    }
    let tid = number(&root, 6);
    let mut out = format!(
        "Android native tombstone summary\nsignal {} ({name}), code {} ({})\ncrashed tid: {tid}\n",
        number(&signal, 1),
        number(&signal, 3) as u32 as i32,
        text(&signal, 4)
    );
    let abort = text(&root, 14);
    if !abort.is_empty() {
        let _ = writeln!(out, "abort message: {abort}");
    }
    let mut count = 0;
    for (id, value) in &root {
        if *id != 16 {
            continue;
        }
        let Value::Bytes(entry) = value else {
            continue;
        };
        let entry = fields(entry);
        if number(&entry, 1) != tid {
            continue;
        }
        let thread = fields(bytes(&entry, 2)?);
        let _ = writeln!(out, "thread: {}", text(&thread, 2));
        for (id, value) in thread {
            if id != 4 || count == 64 {
                continue;
            }
            let Value::Bytes(frame) = value else {
                continue;
            };
            let frame = fields(frame);
            let path = text(&frame, 6);
            let module = path.rsplit('/').next().unwrap_or("unknown");
            let _ = writeln!(
                out,
                "#{count:02} pc {:016x} {module} {}+0x{:x} (BuildId: {})",
                number(&frame, 1),
                text(&frame, 4),
                number(&frame, 5),
                text(&frame, 8)
            );
            count += 1;
        }
        break;
    }
    if count == 0 {
        out.push_str(
            "No crashed-thread frames in the captured prefix; the OS trace may be truncated.\n",
        );
    }
    Some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_input_is_bounded_and_real_fixture_can_be_summarized() {
        for length in 0..300 {
            for byte in [0, 8, 16, 127, 255] {
                let _ = summarize(&vec![byte; length]);
            }
        }
        if let Some(path) = std::env::var_os("HORIZON_NATIVE_TOMBSTONE_FIXTURE") {
            let summary = summarize(&std::fs::read(path).unwrap()).expect("real Android tombstone");
            assert!(summary.contains("SIGABRT"));
            assert!(summary.contains("libc.so"));
            assert!(summary.contains("BuildId:"));
            assert!(!summary.contains("memory_dump"));
        }
    }
}
