use anyhow::Result as Rslt;
use anyhow::anyhow;
use anyhow::bail;
use roxmltree::Document;
use roxmltree::Node;
use std::collections::VecDeque;
use std::path::Path;
const RAML_HEAD: &str = "#%RAML 1.0 Library\n\ntypes:";

#[derive(Debug,)]
pub struct RamlMetadataStream(Vec<RamlTypesMetadata,>,);

impl RamlMetadataStream {
	pub fn new(doc: &Document,) -> Rslt<Self,> {
		let co = get_custom_object(doc,).unwrap();
		let valiant_list = enum_variant_list(&co,);
		let body: Vec<_,> = co
			.children()
			.filter_map(|child| RamlTypesMetadata::new(&child,).ok(),)
			.map(|mut raml_types| {
				let _ = raml_types.set_enum_variant(&valiant_list,);
				raml_types
			},)
			.collect();

		Ok(Self(body,),)
	}

	pub fn filter(&mut self,) {
		todo!()
	}
}

#[derive(PartialEq, Eq, Debug,)]
struct RamlTypesMetadata {
	name:         String,
	type_on_raml: RamlType,
	desc:         String,
	example:      String,
	max_length:   Option<usize,>,
	required:     bool,
}

impl RamlTypesMetadata {
	pub fn new<'a,>(fields: &Node<'a, 'a,>,) -> Rslt<Self,> {
		if fields.tag_name().name() != "fields" {
			bail!("expect node with tagname `fields`")
		}

		let mut name = None;
		let mut type_on_raml = None;
		let mut desc = None;
		let mut example = "".to_string();
		let mut max_length = None;
		let mut required = false;

		fields.children().for_each(|node| {
			let tag_name = node.tag_name().name();
			let text = get_text_of_node(&node,);

			const SFID_LEN: usize = 18;
			match tag_name {
				"fullName" => {
					name.replace(text,);
				},
				"label" => {
					desc.replace(text,);
				},
				"length" => {
					max_length.replace(text.parse::<usize>().expect("failed to get length",),);
				},
				"type" => {
					let rt = match text.as_str() {
						"Lookup" => {
							max_length.replace(SFID_LEN,);
							example = format!("\"{}\"", "X".repeat(SFID_LEN));
							RamlType::String
						},
						// TODO: impl later
						"Picklist" => RamlType::Enum(vec![], Box::new(RamlType::String,),),
						"Number" => {
							example = "0".to_string();
							RamlType::Number
						},
						"Checkbox" => {
							example = "true".to_string();
							RamlType::Boolean
						},
						"Date" => RamlType::Date,
						_a => {
							example = "\"XXX\"".to_string();
							// println!("{_a}");
							RamlType::String
						},
					};
					type_on_raml.replace(rt,);
				},
				"required" => {
					if text.as_str() == "true" {
						required = true;
					}
				},
				// a => unimplemented!("parser for tag with name: `{a}`\n\nnode: {node:?}\n\n"),
				_a => (), //println!("unimplemented tag parser: {a}"),
			};
		},);

		let name = name.unwrap();
		let type_on_raml = type_on_raml.unwrap();
		let desc = desc.unwrap();

		Ok(Self { name, type_on_raml, desc, example, max_length, required, },)
	}

	pub fn format_as_raml(&self,) -> String {
		let mut lines = Vec::with_capacity(4,);
		lines.push(format!("\t{}:", self.name.clone()),);
		lines.push(format!("type: {}", self.type_on_raml.to_string()),);
		lines.push(format!("description: |"),);
		lines.push(format!("\t{}", self.desc.clone()),);
		if let RamlType::Enum(items, _,) = &self.type_on_raml {
			lines.push(format!("enum:"),);
			items.iter().for_each(|item| lines.push(format!("\t- \"{item}\""),),);
		}
		lines.push(format!("example:"),);
		lines.push(format!("\t{}", self.example),);

		let rslt = lines.join("\n\t\t",);
		rslt
	}

	pub fn set_enum_variant(&mut self, variant_list: &Vec<Node,>,) -> Rslt<(),> {
		if let RamlType::Enum(var, _,) = &mut self.type_on_raml {
			let variant = get_enum_variant(variant_list, &self.name,);
			*var = variant;
			self.example = format!("\"{}\"", var[0]);
			Ok((),)
		} else {
			Err(anyhow!("expect RamlType::Enum, found {:?}", self.type_on_raml),)
		}
	}
}

#[derive(PartialEq, Eq, Debug,)]
pub enum RamlType {
	// String(Option<RegexPattern,>,),
	// Number(Option<NumberFormat,>,),
	String,
	Number,
	Enum(Vec<String,>, Box<Self,>,),
	Boolean,
	Date,
}

impl ToString for RamlType {
	fn to_string(&self,) -> String {
		match self {
			RamlType::String => "string".to_string(),
			RamlType::Number => "number".to_string(),
			RamlType::Enum(_items, raml_type,) => raml_type.to_string(),
			RamlType::Boolean => "boolean".to_string(),
			RamlType::Date => "date".to_string(),
		}
	}
}

#[macro_export]
macro_rules! parse_from_path {
	($path:expr,let $name:ident) => {
		let body = $crate::read_file($path,)?;
		let $name = roxmltree::Document::parse(&body,)?;
	};
}

pub fn get_custom_object<'a,>(doc: &'a Document,) -> Option<Node<'a, 'a,>,> {
	let root = doc.root();
	let custom_object = root.first_child()?;
	Some(custom_object,)
}

pub fn get_all_column_metadata<'a,>(doc: &'a Document,) -> Vec<Node<'a, 'a,>,> {
	let co = get_custom_object(doc,).expect("there is no CustomObject under xml",);
	co.children().filter(|child| child.tag_name().name() == "fields",).collect()
}

fn get_text_of_node<'a,>(n: &Node<'a, 'a,>,) -> String {
	n.text().unwrap().to_string()
}

pub fn create_raml_file(data: RamlMetadataStream, filename: impl AsRef<Path,>,) -> Rslt<(),> {
	let mut contents =
		data.0.iter().map(|metadata| metadata.format_as_raml(),).collect::<VecDeque<String,>>();
	contents.push_front(RAML_HEAD.to_string(),);
	let contents = contents.into_iter().collect::<Vec<String,>>().join("\n",);

	std::fs::write(filename, contents,)?;
	Ok((),)
}

fn get_enum_variant<'a,>(variant_list: &Vec<Node,>, name: impl AsRef<str,>,) -> Vec<String,> {
	println!("{}", name.as_ref());
	let target_node = variant_list
		.iter()
		.find(|node| {
			node.children()
				.find(|child| {
					child.tag_name().name() == "picklist" && child.text().unwrap() == name.as_ref()
				},)
				.is_some()
		},)
		.expect(&format!("name {} not found", name.as_ref()),);

	let variants = target_node
		.children()
		.filter(|child| child.tag_name().name() == "values",)
		.map(|child| -> Rslt<String,> {
			let value = child
				.children()
				.find(|child| child.tag_name().name() == "fullName",)
				.unwrap()
				.text()
				.unwrap();
			let value = urlencoding::decode(value,)?.into_owned();
			Ok(value,)
		},)
		.try_collect()
		.unwrap();

	variants
}

fn enum_variant_list<'a,>(custom_object: &Node<'a, 'a,>,) -> Vec<Node<'a, 'a,>,> {
	custom_object
		.children()
		.find(|child| child.tag_name().name() == "recordTypes",)
		.unwrap()
		.children()
		.filter(|child| child.tag_name().name() == "picklistValues",)
		.collect()
}

mod tests {
	#![cfg(test)]

	use crate::read_file;

	use super::*;
	use anyhow::anyhow;

	const OBJ_PATH: &str = "data/IndividualContract__c.object";
	const RAML_ARTICLE_PATH: &str = "data/xxx.raml";

	fn raml_metadata_template() -> Rslt<Vec<RamlTypesMetadata,>,> {
		parse_from_path!(OBJ_PATH, let doc);
		let fields = get_all_column_metadata(&doc,);

		let raml_type = fields_to_raml_metadata(fields,);
		raml_type
	}

	fn raml_data_stream_template() -> Rslt<RamlMetadataStream,> {
		parse_from_path!(OBJ_PATH, let doc);
		RamlMetadataStream::new(&doc,)
	}

	fn fields_to_raml_metadata<'a,>(nodes: Vec<Node<'a, 'a,>,>,) -> Rslt<Vec<RamlTypesMetadata,>,> {
		nodes.iter().map(|node| RamlTypesMetadata::new(node,),).try_collect()
	}

	#[test]
	fn test_read_xml_file() -> Rslt<(),> {
		let _body = read_file(OBJ_PATH,)?;
		Ok((),)
	}

	#[test]
	fn test_parse_from_path() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let rslt);
		assert!(rslt.root().is_root());
		Ok((),)
	}

	#[test]
	fn test_parse_from_path_children() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let root_node = doc.root();
		let first_child = root_node.first_child().unwrap();

		assert_eq!(first_child.tag_name().name(), "CustomObject");
		Ok((),)
	}

	#[test]
	fn test_get_custom_object() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let co = get_custom_object(&doc,).ok_or(anyhow!(""),)?;

		assert_eq!(co.tag_name().name(), "CustomObject");
		Ok((),)
	}

	#[test]
	fn test_get_all_column_metadata() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let fields = get_all_column_metadata(&doc,);

		assert_eq!(fields.len(), 255);

		fields.iter().for_each(|node| assert_eq!(node.tag_name().name(), "fields"),);

		Ok((),)
	}

	#[test]
	fn test_what_is_text_of_node() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let fields = &get_all_column_metadata(&doc,)[0..3];
		let texts = ["Text", "Text", "Lookup",];

		for (field, text,) in fields.iter().zip(texts,) {
			let type_node =
				field.children().find(|node| node.tag_name().name() == "type",).unwrap();
			assert_eq!(type_node.text().unwrap(), text);
		}

		Ok((),)
	}

	#[test]
	fn text_raml_type_metadata_constructor() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let fields = &get_all_column_metadata(&doc,)[0];

		let raml_type = RamlTypesMetadata::new(fields,)?;

		let answer = RamlTypesMetadata {
			name:         "AccessCode__c".to_string(),
			type_on_raml: RamlType::String,
			desc:         "電子契約-アクセスコード".to_string(),
			example:      "\"\"".to_string(),
			max_length:   Some(18,),
			required:     false,
		};

		assert_eq!(answer, raml_type);
		Ok((),)
	}

	#[test]
	fn test_fields_to_raml_metadata() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);

		let fields = get_all_column_metadata(&doc,);
		assert_eq!(fields.len(), 255);
		let raml_datas = fields_to_raml_metadata(fields,)?;
		assert_eq!(raml_datas.len(), 255);

		Ok((),)
	}

	#[test]
	fn test_raml_metadata_format() -> Rslt<(),> {
		let rml_metadata = raml_metadata_template()?;

		let formatted = rml_metadata[0].format_as_raml();
		let answer = r#"	AccessCode__c:
		type: string
		description: |
			電子契約-アクセスコード
		example:
			"""#;
		assert_eq!(formatted, answer);

		Ok((),)
	}

	#[test]
	fn test_raml_metadata_format_with_enum() -> Rslt<(),> {
		let raml_stream = raml_data_stream_template()?;
		let target = raml_stream
			.0
			.iter()
			.find(|raml| matches!(raml.type_on_raml, RamlType::Enum(..)),)
			.unwrap()
			.format_as_raml();
		let answer = r#"	Agreement__c:
		type: string
		description: |
			36協定区分
		enum:
			- "89：一般"
			- "90：一般（フレックス）"
			- "91：一般：延長時間表記有"
			- "92：一般（フレックス）：延長時間表記有"
			- "93：適用除外業務"
			- "94：適用除外業務（フレックス）"
			- "95：総務業務"
			- "96：本社業務"
			- "97：営業職"
			- "98：教育職"
			- "99：教育職（フレックス）"
		example:
			"89：一般""#;

		assert_eq!(target, answer);

		Ok((),)
	}

	#[test]
	fn test_raml_data_stream() -> Rslt<(),> {
		let raml_stream = raml_data_stream_template()?;
		println!("{raml_stream:#?}");
		Ok((),)
	}

	#[test]
	fn test_create_raml_file() -> Rslt<(),> {
		let raml_stream = raml_data_stream_template()?;
		create_raml_file(raml_stream, RAML_ARTICLE_PATH,)?;

		Ok((),)
	}

	#[test]
	fn test_enum_variant_list() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let co = get_custom_object(&doc,).unwrap();
		let evl = enum_variant_list(&co,);

		evl.iter().for_each(|node| {
			println!(
				"name: {}",
				node.children()
					.find(|child| child.tag_name().name() == "picklist")
					.unwrap()
					.text()
					.unwrap()
			)
		},);

		assert_eq!(evl.len(), 44);

		Ok((),)
	}

	#[test]
	fn test_set_enum_variant() -> Rslt<(),> {
		parse_from_path!(OBJ_PATH, let doc);
		let co = get_custom_object(&doc,).unwrap();
		let evl = enum_variant_list(&co,);

		let rml_metadata = raml_metadata_template()?;

		let mut formatted = rml_metadata
			.into_iter()
			.find(|rml| matches!(rml.type_on_raml, RamlType::Enum(..)),)
			.unwrap();
		formatted.set_enum_variant(&evl,)?;

		let RamlType::Enum(variants, _,) = formatted.type_on_raml else { bail!("0w0") };
		println!("{variants:#?}");
		assert_eq!(variants.len(), 11);
		Ok((),)
	}
}
