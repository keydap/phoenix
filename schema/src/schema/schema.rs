use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;

const ATTR_DELIM: &str = ".";
const validTypes: Vec<&str> = vec![
    "string",
    "boolean",
    "decimal",
    "integer",
    "datetime",
    "binary",
    "reference",
    "complex",
];

const validMutability: Vec<&str> = vec!["readonly", "readwrite", "immutable", "writeonly"];

const validReturned: Vec<&str> = vec!["always", "never", "default", "request"];

const validUniqueness: Vec<&str> = vec!["none", "server", "global"];

pub const defaultSchemas: Vec<&str> = vec![
    include_str!("resources/application.json"),
    include_str!("resources/auditevent.json"),
    include_str!("resources/authentication.json"),
    include_str!("resources/device.json"),
    include_str!("resources/enterprise-user.json"),
    include_str!("resources/group.json"),
    include_str!("resources/posix-group.json"),
    include_str!("resources/posix-user.json"),
    include_str!("resources/user.json"),
];

/// The definition of an attribute's type
/// All the fields are named identical to those defined in the schema definition
/// in rfc7643 so that schema JSON files can be parsed using serde
#[derive(Deserialize, Debug)]
pub struct AttrType {
    name: String,                     // name
    Type: String,                     // type
    description: String,              // description
    caseExact: bool,                  // caseExact
    multiValued: bool,                // multiValued
    mutability: String,               // mutability
    required: bool,                   // required
    returned: String,                 // returned
    uniqueness: String,               // uniqueness
    subAttributes: Vec<Rc<AttrType>>, // subAttributes
    referenceTypes: Vec<String>,      // referenceTypes
    schemaId: String,                 // schema's ID
    #[serde(skip)]
    normName: String, // the lowercase name of the attribute
    canonicalValues: Vec<String>,     // canonicalValues
    #[serde(skip)]
    subAttrMap: HashMap<String, Rc<AttrType>>,
    #[serde(skip)]
    parent: Option<Rc<AttrType>>, // parent Attribute, should be non-exportable, otherwise stackoverflow occurs during marshalling
    #[serde(skip)]
    isUnique: bool, // for performance reasons
    #[serde(skip)]
    isComplex: bool, // for performance reasons
    #[serde(skip)]
    isRef: bool, // for performance reasons
    #[serde(skip)]
    isSimple: bool, // for performance reasons
    #[serde(skip)]
    isReadOnly: bool, // for performance reasons
    #[serde(skip)]
    isImmutable: bool, // for performance reasons
    #[serde(skip)]
    isStringType: bool, // for performance reasons
}

#[derive(Deserialize, Debug)]
pub struct Meta {
    location: String,
    resourceType: String,
} // meta

/// Definition of the schema
#[derive(Deserialize, Debug)]
pub struct Schema {
    id: String,
    name: String,
    description: String,
    attributes: Vec<Rc<AttrType>>,
    meta: Meta,
    #[serde(skip)]
    attrMap: HashMap<String, Rc<AttrType>>,
    #[serde(skip)]
    text: String,
    #[serde(skip)]
    uniqueAts: Vec<String>,
    #[serde(skip)]
    requiredAts: Vec<String>,
    #[serde(skip)]
    atsNeverRtn: Vec<String>, // names of attributes that are never returned
    #[serde(skip)]
    atsAlwaysRtn: Vec<String>, // names of attributes that are always returned
    #[serde(skip)]
    atsRequestRtn: Vec<String>, // names of attributes that are returned if requested
    #[serde(skip)]
    atsDefaultRtn: Vec<String>, // names of attributes that are returned by default
    #[serde(skip)]
    atsReadOnly: Vec<String>, // names of attributes that are readonly
}

pub struct SchemaError {
    details: String,
}

impl From<serde_json::error::Error> for SchemaError {
    fn from(e: serde_json::error::Error) -> SchemaError {
        SchemaError {
            details: e.to_string(),
        }
    }
}

impl From<std::io::Error> for SchemaError {
    fn from(e: std::io::Error) -> SchemaError {
        SchemaError {
            details: e.to_string(),
        }
    }
}

/// see section https://tools.ietf.org/html/rfc7643#section-2.2 for the defaults
pub fn new_attr_type() -> AttrType {
    AttrType {
        required: false,
        caseExact: false,
        mutability: String::from("readWrite"),
        returned: String::from("default"),
        uniqueness: String::from("none"),
        Type: String::from("string"),
        name: String::from(""),
        description: String::from(""),
        multiValued: false,
        subAttributes: Vec::new(),
        referenceTypes: Vec::new(),
        canonicalValues: Vec::new(),
        subAttrMap: HashMap::new(),
        schemaId: String::from(""),
        normName: String::from(""),
        parent: None,
        isUnique: false,
        isComplex: false,
        isRef: false,
        isSimple: false,
        isReadOnly: false,
        isImmutable: false,
        isStringType: false,
    }
}

//impl Schema {
//    pub fn load(fileName: &String) -> Result<Schema, SchemaError> {
//        let mut sc: Schema = serde_json::from_str("")?;
//        sc.text = String::from("");
//        Ok(sc)
//    }
//}
/// Parses the given schema file and returns a schema instance after successfuly parsing
pub fn load_schema(fileName: &String) -> Result<Schema, SchemaError> {
    let mut f = File::open(fileName)?;
    let mut data = String::from("");
    f.read_to_string(&mut data)?;

    //log.Infof("Loading schema from file %s", name)
    let mut sc: Schema = serde_json::from_str(&data[..])?;
    sc.text = data;

    for at in sc.attributes.iter_mut() {
        setAttrDefaults(at);
    }

    //let _ve = sc.validate();
    //    let ve = validate(&mut sc);
    //    if !ve.is_empty() {
    //        return Err(SchemaError {
    //            details: String::from(""),
    //        }); //ve.concat()
    //    }

    return Ok(sc);
}

// sets the default values on the missing common fields of schema's attribute type definitions
fn setAttrDefaults(attr: &mut AttrType) {
    if attr.mutability == "" {
        attr.mutability = String::from("readWrite");
    }

    if attr.returned == "" {
        attr.returned = String::from("default");
    }

    if attr.uniqueness == "" {
        attr.uniqueness = String::from("none");
    }

    if attr.Type == "" {
        attr.Type = String::from("string");
    }

    attr.isUnique = (attr.uniqueness == "server") || (attr.uniqueness == "global");
    attr.isComplex = attr.Type == "complex";
    attr.isRef = attr.Type == "reference";
    attr.isSimple = !attr.isComplex && !attr.isRef;
    attr.isReadOnly = attr.mutability == "readonly";
    attr.isImmutable = attr.mutability == "immutable";
    attr.isStringType = attr.Type == "string" || attr.isRef;

    for at in attr.subAttributes.iter_mut() {
        setAttrDefaults(at);
    }
}

fn validate(sc: &mut Schema) -> Vec<&str> {
    let mut ve = Vec::new();

    if sc.id == "" {
        ve.push("Schema id is required");
    }

    if sc.attributes.len() == 0 {
        ve.push("A schema should contain atleast one attribute");
        return ve;
    }

    let validNameRegex: Regex = Regex::new(r"^[0-9A-Za-z_$-]+$").unwrap();
    for attr in sc.attributes.iter_mut() {
        sc.validateAttrType(attr, &mut ve, &validNameRegex);
        let name = attr.name.to_ascii_lowercase();
        sc.attrMap.insert(name.clone(), Rc::clone(attr));
        if attr.isUnique {
            sc.uniqueAts.push(name.clone())
        }

        if attr.required {
            sc.requiredAts.push(name.clone())
        }
    }

    return ve;
}

fn validateAttrType(
    sc: &Schema,
    attr: Rc<AttrType>,
    ve: &mut Vec<&str>,
    validNameRegex: &Regex,
) -> (Vec<String>, Vec<String>) {
    // ATTRNAME   = ALPHA *(nameChar)
    // nameChar   = "$" / "-" / "_" / DIGIT / ALPHA
    // ALPHA      =  %x41-5A / %x61-7A   ; A-Z / a-z
    // DIGIT      =  %x30-39            ; 0-9

    if !validNameRegex.is_match(&attr.name) {
        ve.push(&format!("invalid attribute name '{}'", &attr.name));
    }

    attr.Type.make_ascii_lowercase();
    if !exists(&attr.Type, validTypes) {
        ve.push(&format!(
            "invalid type '{}' for attribute '{}'",
            &attr.Type, &attr.name
        ));
    }

    attr.mutability.make_ascii_lowercase();
    if !exists(&attr.mutability, validMutability) {
        ve.push(&format!(
            "invalid mutability '{}' for attribute '{}'",
            &attr.mutability, &attr.name
        ));
    }

    attr.returned.make_ascii_lowercase();
    if !exists(&attr.returned, validReturned) {
        ve.push(&format!(
            "invalid returned '{}' for attribute '{}'",
            &attr.returned, &attr.name
        ));
    }

    attr.uniqueness.make_ascii_lowercase();
    if !exists(&attr.uniqueness, validUniqueness) {
        ve.push(&format!(
            "invalid uniqueness '{}' for attribute '{}'",
            &attr.uniqueness, &attr.name
        ));
    }

    if attr.isRef && (attr.referenceTypes.len() == 0) {
        ve.push(&format!(
            "No referenceTypes set for attribute '{}'",
            &attr.name
        ));
    }

    if attr.isComplex && (attr.subAttributes.len() == 0) {
        ve.push(&format!(
            "No subattributes set for attribute '{}'",
            &attr.name
        ));
    }

    attr.schemaId = sc.id.clone();
    attr.normName = attr.name.to_ascii_lowercase();

    let mut uniqueAts: Vec<String> = vec![];
    let mut requiredAts: Vec<String> = vec![];

    if attr.isComplex {
        //log.Debugf("validating sub-attributes of attributetype %s\n", attr.Name)
        for sa in attr.subAttributes {
            //log.Tracef("validating sub-type %s of %s", sa.Name, attr.Name);
            let mut a = &sa;
            validateAttrType(sc, sa, ve, validNameRegex);
            a.parent = Some(Rc::clone(attr));
            attr.subAttrMap.insert(sa.normName.clone(), Rc::clone(a));
            let name = format!("{}{}{}", &attr.normName, ATTR_DELIM, &sa.normName);
            if sa.isUnique {
                uniqueAts.push(name.clone());
            }
            if sa.required {
                requiredAts.push(name)
            }
        }

        // add missing default sub-attributes https://tools.ietf.org/html/rfc7643#section-2.4
        if attr.multiValued {
            addDefSubAttrs(attr);
            setAttrDefaults(attr);
        }
    }
}

fn exists(val: &String, list: Vec<&str>) -> bool {
    for token in list.iter() {
        if token == val {
            return true;
        }
    }
    return false;
}

fn addDefSubAttrs(attr: &mut AttrType) {
    let mut defArr = Vec::new();

    let mut typeAttr = new_attr_type();
    typeAttr.name = String::from("type");
    typeAttr.normName = typeAttr.name.clone();
    defArr.push(typeAttr);

    let mut primaryAttr = new_attr_type();
    primaryAttr.name = String::from("primary");
    primaryAttr.normName = primaryAttr.name.clone();
    primaryAttr.Type = String::from("boolean");
    defArr.push(primaryAttr);

    let mut displayAttr = new_attr_type();
    displayAttr.name = String::from("display");
    displayAttr.normName = displayAttr.name.clone();
    displayAttr.mutability = String::from("immutable");
    defArr.push(displayAttr);

    let mut valueAttr = new_attr_type();
    valueAttr.name = String::from("value");
    valueAttr.normName = valueAttr.name.clone();
    defArr.push(valueAttr);

    let mut refAttr = new_attr_type();
    refAttr.name = String::from("$ref");
    refAttr.normName = refAttr.name.clone();
    defArr.push(refAttr);

    for a in defArr.iter_mut() {
        let key = a.name.to_ascii_lowercase();
        if !attr.subAttrMap.contains_key(&key) {
            a.schemaId = attr.schemaId.clone();
            a.parent = Some(attr);
            attr.subAttrMap.insert(key, a);
        }
    }
}
