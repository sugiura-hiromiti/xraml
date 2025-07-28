use crate::read_file;
use anyhow::Result as Rslt;
use anyhow::anyhow;
use anyhow::bail;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

const PROPERTY_FILE_HEADER: &str = "name,example";

#[derive(Debug,)]
pub struct Csv {
	rows:               CsvRows,
	pub target_columns: Vec<usize,>,
}

impl Csv {
	pub fn filter_map<R: std::iter::FromIterator<std::string::String,>,>(
		&self,
		condition: impl FnMut(Vec<String,>,) -> Option<String,>,
	) -> R {
		self.rows.clone().filter_map(condition,).collect()
	}

	pub fn acquire_required_rows_name(&self,) -> Vec<String,> {
		let condition = |v: Vec<String,>| {
			let is_required = v[self.target_columns[1]..].iter().any(|s| s.contains("〇",),);
			if is_required { Some(v[self.target_columns[0]].clone(),) } else { None }
		};

		let rslt = self.filter_map(condition,);
		rslt
	}

	pub fn update_property_file(self,) -> Rslt<String,> {
		let content = read_property_file()?;
		let content = self.update_property_file_content(content,)?;
		write_proterty_file(&content,)?;
		Ok(content,)
	}

	pub fn update_property_file_content(&self, content: String,) -> Rslt<String,> {
		let required_rows = self.acquire_required_rows_name();

		let mut content: Vec<String,> = content.split('\n',).map(|s| s.to_string(),).collect();
		let len = if content.is_empty() {
			content.push(PROPERTY_FILE_HEADER.to_string(),);
			1
		} else {
			content.len()
		};

		for row in required_rows {
			if len == 1 {
				content.push(property_file_line_format(&row, None::<&str,>,),);
			}

			for i in 1..len {
				if !content[i].contains(&row,) {
					let new_line = property_file_line_format(&row, None::<&str,>,);
					content.push(new_line,);
				}
			}
		}

		let rslt = content.join("\n",);
		Ok(rslt,)
	}
}

#[derive(Debug, Clone,)]
pub struct CsvRows {
	data:        Vec<String,>,
	current_row: usize,
}

impl Iterator for CsvRows {
	type Item = Vec<String,>;

	fn next(&mut self,) -> Option<Self::Item,> {
		self.current_row += 1;

		if self.current_row >= self.data.len() {
			return None;
		}

		let next_row: Vec<String,> =
			self.data[self.current_row].split(',',).map(|s| s.to_string(),).collect();

		if next_row[0] != "" || next_row[1].parse::<i32>().is_err() {
			return self.next();
		}

		Some(next_row,)
	}
}

pub fn read_as_csv(path: impl AsRef<Path,>,) -> Rslt<Csv,> {
	let contents = read_file(path,)?;
	let (_, post,) =
		contents.split_once("項目一覧",).ok_or(anyhow!("csv file has unexpected format"),)?;

	let mut lines = post.lines();
	lines.next();
	let mut had_csv = false;
	let header: Vec<&str,> = lines
		.take_while(|s| {
			!if had_csv {
				true
			} else {
				let has_csv = s.contains("CSV",);
				if has_csv {
					had_csv = true;
				}
				false
			}
		},)
		.collect();

	let header_count = header.len();
	let target_columns = header
		.join("",)
		.split(',',)
		.enumerate()
		.filter_map(|(i, name,)| {
			if name.contains("CSV",) || name.contains("API参照名",) { Some(i,) } else { None }
		},)
		.collect();

	println!("target_columns: {target_columns:?}");

	let mut data = Vec::with_capacity(400,);
	let lines =
		post.lines().skip(header_count,).skip_while(|s| s.split_once(',',).unwrap().0 != "",);
	// .filter_map(|s| s.contains(|c| c != ',',).then_some(s.to_string(),),)
	// .collect();

	let mut row = "".to_string();
	for line in lines {
		if line.starts_with(',',) {
			data.push(row,);
			row = line.to_string();
		} else {
			row.push_str(line,);
		}
	}
	let rows = CsvRows { data, current_row: 0, };

	Ok(Csv { rows, target_columns, },)
}

pub fn property_file_line_format(name: impl Display, example: Option<impl Display,>,) -> String {
	match example {
		Some(example,) => format!("{name},{example}"),
		None => format!("{name},"),
	}
}

pub fn open_property_file(read: bool, write: bool,) -> Rslt<File,> {
	if !read && !write {
		bail!("invalid argument. both read/write are false")
	}
	let mut file_open_opts = std::fs::OpenOptions::new();
	file_open_opts.read(read,).write(write,).create(write,);
	let file = file_open_opts.open("data/property.csv",)?;
	Ok(file,)
}

pub fn read_property_file() -> Rslt<String,> {
	let mut file = open_property_file(true, false,)?;
	let mut content = String::new();
	file.read_to_string(&mut content,)?;
	Ok(content,)
}

pub fn write_proterty_file(content: &String,) -> Rslt<(),> {
	let mut file = open_property_file(false, true,)?;
	file.write_all(content.as_bytes(),)?;
	Ok((),)
}

#[cfg(test)]
mod tests {
	use super::*;

	const KOBETU_CSV_PATH: &str = "/Users/hiromichi.sugiura/Downloads/ws/xraml/data/kobetu.csv";
	const KEIYAKU_CSV_PATH: &str = "/Users/hiromichi.sugiura/Downloads/ws/xraml/data/keiyaku.csv";

	fn csv_template() -> Rslt<Csv,> {
		read_as_csv(KOBETU_CSV_PATH,)
	}

	fn keiyaku_csv_template() -> Rslt<Csv,> {
		read_as_csv(KEIYAKU_CSV_PATH,)
	}

	#[test]
	fn test_read_as_csv() -> Rslt<(),> {
		let _csv = csv_template()?;
		Ok((),)
	}

	#[test]
	fn test_csv_iter() -> Rslt<(),> {
		let csv = csv_template()?;

		let voids: Vec<Vec<String,>,> = csv.rows.into_iter().collect();
		assert_eq!(voids.len(), 347, "{voids:#?}");

		voids.iter().for_each(|s| assert!(!s.is_empty()),);
		Ok((),)
	}

	#[test]
	fn test_acquire_required() -> Rslt<(),> {
		let csv = csv_template()?;

		let rslt: Vec<String,> = csv.acquire_required_rows_name();
		assert_eq!(rslt.len(), 104);

		let ano = keiyaku_csv_template()?;
		let rslt: Vec<String,> = ano.acquire_required_rows_name();
		assert_eq!(rslt.len(), 77);
		Ok((),)
	}

	#[test]
	fn test_open_pfile() -> Rslt<(),> {
		let _f = open_property_file(true, true,)?;
		let _f = open_property_file(false, true,)?;
		let _f = open_property_file(true, false,)?;
		Ok((),)
	}

	#[test]
	#[should_panic]
	fn test_open_pfile_with_invalid_argument() {
		open_property_file(false, false,).unwrap();
	}
}
