use regex::Regex;
use std::collections::{HashMap};
use std::fs::File;
use std::io::{Read, Error};
use serde::{Deserialize};

const ATTR_DELIM: &str = ".";
const validTypes: Vec<&str> = vec!["string", "boolean", "decimal", "integer", "datetime", "binary", "reference", "complex"];

const validMutability: Vec<&str> = vec!["readonly", "readwrite", "immutable", "writeonly"];

const validReturned: Vec<&str> = vec!["always", "never", "default", "request"];

const validUniqueness: Vec<&str> = vec!["none", "server", "global"];

const validNameRegex: Regex = Regex::new(r"^[0-9A-Za-z_$-]+$").unwrap();

/// The definition of an attribute's type
/// All the fields are named identical to those defined in the schema definition
/// in rfc7643 so that schema JSON files can be parsed using serde
#[derive(Deserialize, Debug)]
pub struct AttrType<'a> {
    name: String,      // name
    Type: String,      // type
    description: String,      // description
    caseExact: bool,        // caseExact
    multiValued: bool,        // multiValued
    mutability: String,      // mutability
    required: bool,        // required
    returned: String,      // returned
    uniqueness: String,      // uniqueness
    subAttributes:  Vec<AttrType<'a>>, // subAttributes
    referenceTypes:  Vec<String>,    // referenceTypes
    schemaId: String,    // schema's ID
    #[serde(skip)]
    normName: String,    // the lowercase name of the attribute
    canonicalValues: Vec<String>,    // canonicalValues
    #[serde(skip)]
    subAttrMap:      HashMap<String, &'a AttrType<'a>>,
    #[serde(skip)]
    parent:   Option<&'a AttrType<'a>>, // parent Attribute, should be non-exportable, otherwise stackoverflow occurs during marshalling
    #[serde(skip)]
    isUnique: bool,      // for performance reasons
    #[serde(skip)]
    isComplex: bool,      // for performance reasons
    #[serde(skip)]
    isRef: bool,      // for performance reasons
    #[serde(skip)]
    isSimple: bool,      // for performance reasons
    #[serde(skip)]
    isReadOnly: bool,      // for performance reasons
    #[serde(skip)]
    isImmutable: bool,      // for performance reasons
    #[serde(skip)]
    isStringType: bool      // for performance reasons
}

#[derive(Deserialize, Debug)]
pub struct Meta {
    location:     String,
    resourceType: String
} // meta

/// Definition of the schema
#[derive(Deserialize, Debug)]
pub struct Schema<'a> {
    id:          String,
    name:        String,
    description: String,
    attributes:  Vec<AttrType<'a>>,
    meta: Meta,
    #[serde(skip)]
    attrMap:     HashMap<String, &'a AttrType<'a>>,
    #[serde(skip)]
    text:        String,
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
    details: String
}

impl From<Error> for SchemaError {
    fn from(e: Error) -> SchemaError {
        SchemaError{details: e.to_string()}
    }
}

/// see section https://tools.ietf.org/html/rfc7643#section-2.2 for the defaults
pub fn new_attr_type<'a>() -> AttrType<'a> {
  AttrType{required: false, caseExact: false, mutability: String::from("readWrite"),
      returned: String::from("default"), uniqueness: String::from("none"), Type: String::from("string"),
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
      isStringType: false
  }
}

/// Parses the given schema file and returns a schema instance after successfuly parsing
pub fn load_schema(fileName: &String) -> Result<Schema, SchemaError> {
    let mut f = File::open(fileName)?;
    let mut data= String::from("");
    f.read_to_string(&mut data)?;

    //log.Infof("Loading schema from file %s", name)
    let mut sc: Schema = serde_json::from_str(&data[..])?;

    for at in sc.attributes.iter_mut() {
        setAttrDefaults(at);
    }

    let ve = validate(&mut sc);
    if !ve.is_empty() {
        return Err(SchemaError{details: String::from("")});//ve.concat()
    }

    sc.text = data;

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

fn validate<'a>(sc: &mut Schema) -> Vec<&'a str> {
    let mut ve = Vec::new();

    if sc.id == "" {
        ve.push("Schema id is required");
    }

    if sc.attributes.len() == 0 {
        ve.push("A schema should contain atleast one attribute");
        return ve;
    }

    for attr in sc.attributes.iter_mut() {
        validateAttrType(attr, sc, &mut ve);
        let name = attr.name.to_ascii_lowercase();
        sc.attrMap.insert(name.clone(), &attr);
        if attr.IsUnique() {
            sc.uniqueAts.push(name.clone())
        }

        if attr.required {
            sc.requiredAts.push(name.clone())
        }
    }

    return ve
}

fn validateAttrType(attr: &mut AttrType, sc: &mut Schema, ve: &mut Vec<&str>) {
    // ATTRNAME   = ALPHA *(nameChar)
    // nameChar   = "$" / "-" / "_" / DIGIT / ALPHA
    // ALPHA      =  %x41-5A / %x61-7A   ; A-Z / a-z
    // DIGIT      =  %x30-39            ; 0-9

    if !validNameRegex.is_match(&attr.name) {
        ve.push("Invalid attribute name '" + &attr.name + "'");
    }

    attr.Type.make_ascii_lowercase();
    if !exists(&attr.Type, validTypes) {
        ve.push("Invalid type '" + &attr.Type + "' for attribute " + &attr.name);
    }

    attr.mutability.make_ascii_lowercase();
    if !exists(&attr.mutability, validMutability) {
        ve.push("Invalid mutability '" + &attr.mutability + "' for attribute " + &attr.name);
    }

    attr.returned.make_ascii_lowercase();
    if !exists(&attr.returned, validReturned) {
        ve.push("Invalid returned '" + &attr.returned + "' for attribute " + &attr.name);
    }

    attr.uniqueness = String::to_lowercase(&attr.uniqueness);
    if !exists(&attr.uniqueness, validUniqueness) {
        ve.push("Invalid uniqueness '" + &attr.uniqueness + "' for attribute " + &attr.name);
    }

    if attr.IsRef() && (attr.referenceTypes.len() == 0) {
        ve.push("No referenceTypes set for attribute " + &attr.name);
    }

    if attr.IsComplex() && (attr.subAttributes.len() == 0) {
        ve.push("No subattributes set for attribute " + &attr.name);
    }

    attr.schemaId = sc.Id;
    attr.normName = String::to_lowercase(&attr.name);

    if attr.IsComplex() {
        //log.Debugf("validating sub-attributes of attributetype %s\n", attr.Name)
        for sa in attr.subAttributes.iter_mut() {
            //log.Tracef("validating sub-type %s of %s", sa.Name, attr.Name);
            validateAttrType(sa, sc, ve);
            sa.parent = Some(attr);
            attr.subAttrMap.insert(sa.normName.clone(), sa);
            let name = &attr.normName + ATTR_DELIM + &sa.normName;
            if sa.IsUnique() {
                sc.uniqueAts.push(name);
            }
            if sa.Required {
                sc.requiredAts.push(name)
            }
        }

        // add missing default sub-attributes https://tools.ietf.org/html/rfc7643#section-2.4
        if attr.MultiValued {
            addDefSubAttrs(attr);
            setAttrDefaults(attr);
        }
    }
}

fn exists(val: &String, list: Vec<&str>) -> bool {
    for token in list.iter() {
        if token == val {
            return true
        }
    }
    return false
}

fn addDefSubAttrs(attr: & mut AttrType) {
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
    displayAttr.Mutability = String::from("immutable");
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
        let key = String::to_lowercase(&a.name);
        if !attr.subAttrMap.contains_key(key) {
            a.SchemaId = attr.schemaId.clone();
            a.parent = Some(attr);
            attr.subAttrMap.insert(key, a);
        }
    }
}
