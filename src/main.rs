use anyhow::Result as Rslt;
use xraml::csv::read_as_csv;
use xraml::parse_from_path;
use xraml::raml::get_all_column_metadata;

fn main() -> Rslt<(),> {
	let content = read_as_csv(
		"/Users/hiromichi.sugiura/Downloads/ws/xraml/data/columns_of_individual_contract.csv",
	)?;

	let condition = |v: Vec<String,>| {
		let is_required = v[content.target_columns[1]].contains("〇",);
		if is_required { Some(v[content.target_columns[0]].clone(),) } else { None }
	};

	let rslt: Vec<String,> = content.filter_map(condition,);
	println!("{rslt:#?}");
	Ok((),)
}
