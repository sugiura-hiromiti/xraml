use anyhow::Result as Rslt;
use xraml::csv::read_as_csv;
use xraml::raml::create_raml_metadata_stream;

const INDIVIDUAL_CONTRACT_OBJ_PATH: &str = "data/IndividualContract__c.object";
const SOEC_OBJ_PATH: &str = "data/SalesOrderEmploymentConditions__c.object";
const IC_RAML: &str = "individual_contract.raml";
const SOEC_RAML: &str = "sales_order_employment_conditions.raml";
const IC_CSV: &str = "data/kobetu.csv";
const SOEC_CSV: &str = "data/keiyaku.csv";

fn main() -> Rslt<(),> {
	let content = vec![
		(read_as_csv(IC_CSV,)?, INDIVIDUAL_CONTRACT_OBJ_PATH, IC_RAML,),
		(read_as_csv(SOEC_CSV,)?, SOEC_OBJ_PATH, SOEC_RAML,),
	];

	for (csv, obj_path, raml_file,) in content {
		let acquired_rows = csv.acquire_required_rows_name();
		create_raml_metadata_stream(obj_path,)?
			.create_raml_file_minimal(acquired_rows, raml_file.to_string(),)?;
	}

	Ok((),)
}
