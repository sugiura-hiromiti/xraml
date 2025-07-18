#![feature(iterator_try_collect)]

//  TODO: - [x] required, null指定はいらない

pub mod csv;
pub mod raml;

use anyhow::Result as Rslt;
use std::path::Path;

/// ```should_panic
/// let rslt = xraml::read_file("abc",).unwrap();
/// ```
pub fn read_file(path: impl AsRef<Path,>,) -> Rslt<String,> {
	let body = std::fs::read_to_string(path,)?;
	Ok(body,)
}
