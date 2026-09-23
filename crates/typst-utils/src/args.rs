/// Parses a key/value pair split by the first equal sign.
///
/// This function will return an error if the argument contains no equals sign
/// or if the key (before the equals sign) is empty.
///
/// The intended usage of this function is to parse `--input key=value` on the
/// CLI for `sys.inputs`.
pub fn parse_sys_input_pair(raw: &str) -> Result<(String, String), String> {
    let (key, val) = raw
        .split_once('=')
        .ok_or("input must be a key and a value separated by an equal sign")?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("the key was missing or empty".to_owned());
    }
    let val = val.trim().to_owned();
    Ok((key, val))
}
