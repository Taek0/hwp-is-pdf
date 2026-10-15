//! Field commands and bookmark parameter strings from HWP5 tables 50–52/152.
use super::*;
pub(super) fn field(r: &Record) -> Result<Node> {
    let source = r.id();
    let d = &r.data;
    let kind = match source.as_str() {
        "%hlk" => "HYPERLINK",
        "%bmk" => "BOOKMARK",
        "%xrf" => "CROSSREF",
        "%smr" => "SUMMERY",
        "%dte" => "DATE",
        "%ddt" => "DOC_DATE",
        "%pat" => "PATH",
        "%fmu" => "FORMULA",
        "%toc" => "TABLEOFCONTENTS",
        _ => "UNKNOWN",
    };
    let mut out = n("fieldBegin")
        .at("type", kind)
        .at("sourceControl", &source)
        .at("id", format!("hwp-{:x}", r.offset));
    if d.len() >= 11 {
        let count = u16(d, 9)? as usize;
        if 15 + count * 2 <= d.len() {
            out = out
                .at("command", utf16(bytes(d, 11, count * 2)?)?)
                .at("id", u32(d, 11 + count * 2)?);
        } else {
            out = out.at("metadataIncomplete", 1);
        }
    } else {
        out = out.at("metadataIncomplete", 1);
    }
    Ok(n("ctrl").with(vec![out]))
}
fn strings(d: &[u8], off: &mut usize, depth: usize, out: &mut Vec<String>) -> Result<()> {
    anyhow::ensure!(depth < 16, "Excessive bookmark parameter nesting");
    let count = u16(d, *off + 2)? as usize;
    *off += 4;
    anyhow::ensure!(count <= 1024, "Excessive bookmark parameters");
    for _ in 0..count {
        let kind = u16(d, *off + 2)?;
        *off += 4;
        match kind {
            1 => {
                let len = u16(d, *off)? as usize;
                *off += 2;
                out.push(
                    utf16(bytes(d, *off, len * 2)?)?
                        .trim_end_matches('\0')
                        .into(),
                );
                *off += len * 2;
            }
            0 | 2..=9 => {
                bytes(d, *off, 4)?;
                *off += 4;
            }
            0x8000 => strings(d, off, depth + 1, out)?,
            0x8002 => {
                bytes(d, *off, 2)?;
                *off += 2;
            }
            _ => bail!("Unsupported bookmark parameter type {kind}"),
        }
    }
    Ok(())
}
pub(super) fn bookmark(r: &Record) -> Result<Node> {
    let mut records = Vec::new();
    r.all(&mut records);
    let mut values = Vec::new();
    let parsed = records
        .into_iter()
        .find(|r| r.tag == 87)
        .is_some_and(|data| strings(&data.data, &mut 0, 0, &mut values).is_ok());
    let name = values
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    Ok(n("ctrl").with(vec![
        n("bookmark")
            .at("name", name)
            .at("metadataIncomplete", usize::from(!parsed)),
    ]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_and_parameter_strings() {
        let command = "https://example.com";
        let mut data = vec![0; 11];
        data[..4].copy_from_slice(b"klh%");
        data[9..11].copy_from_slice(&(command.encode_utf16().count() as u16).to_le_bytes());
        for c in command.encode_utf16() {
            data.extend(c.to_le_bytes());
        }
        data.extend(7u32.to_le_bytes());
        let r = Record {
            tag: 71,
            level: 0,
            data,
            offset: 0,
            children: Vec::new(),
        };
        let node = field(&r).unwrap();
        assert_eq!(node.child("fieldBegin").unwrap().a("command"), command);
        let mut p = vec![0, 0, 1, 0, 0, 0, 1, 0, 3, 0];
        for c in "목차A".encode_utf16() {
            p.extend(c.to_le_bytes());
        }
        let mut out = Vec::new();
        strings(&p, &mut 0, 0, &mut out).unwrap();
        assert_eq!(out, vec!["목차A"]);
    }
}
