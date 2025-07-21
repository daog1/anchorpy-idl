//use crate::idlv00 as IdlV00;
use crate::spec::idlv00 as IV00;
use anchor_lang_idl_spec as t;
use anchor_lang_idl_spec::Idl as IdlV01;
use anyhow::{anyhow, Result};
use heck::ToSnakeCase;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Enum representing different IDL versions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IdlVersion {
    V00,
    V01,
}
/// Detect the IDL version from JSON value
pub fn detect_idl_version(json: &Value) -> IdlVersion {
    // v0.1 has 'metadata' field with 'spec' inside
    if let Some(metadata) = json.get("metadata") {
        if metadata.get("spec").is_some() {
            return IdlVersion::V01;
        }
    }

    // v0.1 has 'address' field at root level
    //if json.get("address").is_some() {
    //    return IdlVersion::V01;
    //}

    // v0.0 has 'version' and 'name' at root level without 'metadata'
    if json.get("version").is_some() && json.get("name").is_some() && json.get("metadata").is_none()
    {
        return IdlVersion::V00;
    }

    // Check for v0.0 specific account structure (is_mut, is_signer)
    if let Some(instructions) = json.get("instructions") {
        if let Some(instruction) = instructions.as_array().and_then(|arr| arr.first()) {
            if instruction.get("discriminator").is_none() {
                return IdlVersion::V00;
            }
            if let Some(accounts) = instruction.get("accounts") {
                if let Some(account) = accounts.as_array().and_then(|arr| arr.first()) {
                    if account.get("is_mut").is_some() || account.get("is_signer").is_some()
                        |account.get("is_Mut").is_some() || account.get("is_Signer").is_some()
                        | account.get("isMut").is_some() || account.get("isSigner").is_some() {
                        return IdlVersion::V00;
                    }
                }
            }
        }
    }

    // Default to v0.1 if uncertain
    IdlVersion::V01
}

impl TryFrom<IV00::Idl> for t::Idl {
    type Error = anyhow::Error;

    fn try_from(idl: IV00::Idl) -> Result<Self> {
        Ok(Self {
            address: {
                let addr = idl
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("address"))
                    .and_then(|a| a.as_str());
                if addr.is_none() {
                    eprintln!("Warning: Program id missing in `idl.metadata.address` field");
                }
                addr.unwrap_or("").into()
            },
            metadata: t::IdlMetadata {
                name: idl.name,
                version: idl.version,
                spec: t::IDL_SPEC.into(),
                description: Default::default(),
                repository: Default::default(),
                dependencies: Default::default(),
                contact: Default::default(),
                deployments: Default::default(),
            },
            docs: idl.docs.unwrap_or_default(),
            instructions: idl.instructions.into_iter().map(Into::into).collect(),
            accounts: idl.accounts.clone().into_iter().map(Into::into).collect(),
            events: idl
                .events
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            errors: idl
                .errors
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            types: idl
                .types
                .into_iter()
                .map(Into::into)
                .chain(idl.accounts.into_iter().map(Into::into))
                .chain(idl.events.unwrap_or_default().into_iter().map(Into::into))
                .collect(),
            constants: idl.constants.into_iter().map(Into::into).collect(),
        })
    }
}

fn get_disc(prefix: &str, name: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prefix);
    hasher.update(b":");
    hasher.update(name);
    hasher.finalize()[..8].into()
}

impl From<IV00::IdlInstruction> for t::IdlInstruction {
    fn from(value: IV00::IdlInstruction) -> Self {
        let name = value.name.to_snake_case();
        Self {
            discriminator: get_disc("global", &name),
            name,
            docs: value.docs.unwrap_or_default(),
            accounts: value.accounts.into_iter().map(Into::into).collect(),
            args: value.args.into_iter().map(Into::into).collect(),
            returns: value.returns.map(|r| r.into()),
        }
    }
}

impl From<IV00::IdlTypeDefinition> for t::IdlAccount {
    fn from(value: IV00::IdlTypeDefinition) -> Self {
        Self {
            discriminator: get_disc("account", &value.name),
            name: value.name,
        }
    }
}

impl From<IV00::IdlEvent> for t::IdlEvent {
    fn from(value: IV00::IdlEvent) -> Self {
        Self {
            discriminator: get_disc("event", &value.name),
            name: value.name,
        }
    }
}

impl From<IV00::IdlErrorCode> for t::IdlErrorCode {
    fn from(value: IV00::IdlErrorCode) -> Self {
        Self {
            name: value.name,
            code: value.code,
            msg: value.msg,
        }
    }
}

impl From<IV00::IdlConst> for t::IdlConst {
    fn from(value: IV00::IdlConst) -> Self {
        Self {
            name: value.name,
            docs: Default::default(),
            ty: value.ty.into(),
            value: value.value,
        }
    }
}

impl From<IV00::IdlDefinedTypeArg> for t::IdlGenericArg {
    fn from(value: IV00::IdlDefinedTypeArg) -> Self {
        match value {
            IV00::IdlDefinedTypeArg::Type(ty) => Self::Type { ty: ty.into() },
            IV00::IdlDefinedTypeArg::Value(value) => Self::Const { value },
            IV00::IdlDefinedTypeArg::Generic(generic) => Self::Type {
                ty: t::IdlType::Generic(generic),
            },
        }
    }
}

impl From<IV00::IdlTypeDefinition> for t::IdlTypeDef {
    fn from(value: IV00::IdlTypeDefinition) -> Self {
        Self {
            name: value.name,
            docs: value.docs.unwrap_or_default(),
            serialization: Default::default(),
            repr: Default::default(),
            generics: Default::default(),
            ty: value.ty.into(),
        }
    }
}

impl From<IV00::IdlEvent> for t::IdlTypeDef {
    fn from(value: IV00::IdlEvent) -> Self {
        Self {
            name: value.name,
            docs: Default::default(),
            serialization: Default::default(),
            repr: Default::default(),
            generics: Default::default(),
            ty: t::IdlTypeDefTy::Struct {
                fields: Some(t::IdlDefinedFields::Named(
                    value
                        .fields
                        .into_iter()
                        .map(|f| t::IdlField {
                            name: f.name.to_snake_case(),
                            docs: Default::default(),
                            ty: f.ty.into(),
                        })
                        .collect(),
                )),
            },
        }
    }
}

impl From<IV00::IdlTypeDefinitionTy> for t::IdlTypeDefTy {
    fn from(value: IV00::IdlTypeDefinitionTy) -> Self {
        match value {
            IV00::IdlTypeDefinitionTy::Struct { fields } => Self::Struct {
                fields: if fields.is_empty() {
                    None
                } else {
                    Some(t::IdlDefinedFields::Named(
                        fields.into_iter().map(Into::into).collect(),
                    ))
                },
            },
            IV00::IdlTypeDefinitionTy::Enum { variants } => Self::Enum {
                variants: variants
                    .into_iter()
                    .map(|variant| t::IdlEnumVariant {
                        name: variant.name,
                        fields: variant.fields.map(|fields| match fields {
                            IV00::EnumFields::Named(fields) => t::IdlDefinedFields::Named(
                                fields.into_iter().map(Into::into).collect(),
                            ),
                            IV00::EnumFields::Tuple(tys) => t::IdlDefinedFields::Tuple(
                                tys.into_iter().map(Into::into).collect(),
                            ),
                        }),
                    })
                    .collect(),
            },
            IV00::IdlTypeDefinitionTy::Alias { value } => Self::Type {
                alias: value.into(),
            },
        }
    }
}

impl From<IV00::IdlField> for t::IdlField {
    fn from(value: IV00::IdlField) -> Self {
        Self {
            name: value.name.to_snake_case(),
            docs: value.docs.unwrap_or_default(),
            ty: value.ty.into(),
        }
    }
}

impl From<IV00::IdlType> for t::IdlType {
    fn from(value: IV00::IdlType) -> Self {
        match value {
            IV00::IdlType::PublicKey => t::IdlType::Pubkey,
            IV00::IdlType::Defined(name) => t::IdlType::Defined {
                name,
                generics: Default::default(),
            },
            IV00::IdlType::DefinedWithTypeArgs { name, args } => t::IdlType::Defined {
                name,
                generics: args.into_iter().map(Into::into).collect(),
            },
            IV00::IdlType::Option(ty) => t::IdlType::Option(ty.into()),
            IV00::IdlType::Vec(ty) => t::IdlType::Vec(ty.into()),
            IV00::IdlType::Array(ty, len) => {
                t::IdlType::Array(ty.into(), t::IdlArrayLen::Value(len))
            }
            IV00::IdlType::GenericLenArray(ty, generic) => {
                t::IdlType::Array(ty.into(), t::IdlArrayLen::Generic(generic))
            }
            _ => serde_json::to_value(value)
                .and_then(serde_json::from_value)
                .unwrap(),
        }
    }
}

impl From<Box<IV00::IdlType>> for Box<t::IdlType> {
    fn from(value: Box<IV00::IdlType>) -> Self {
        Box::new((*value).into())
    }
}

impl From<IV00::IdlAccountItem> for t::IdlInstructionAccountItem {
    fn from(value: IV00::IdlAccountItem) -> Self {
        match value {
            IV00::IdlAccountItem::IdlAccount(acc) => Self::Single(t::IdlInstructionAccount {
                name: acc.name.to_snake_case(),
                docs: acc.docs.unwrap_or_default(),
                writable: acc.is_mut,
                signer: acc.is_signer,
                optional: acc.is_optional.unwrap_or_default(),
                address: Default::default(),
                pda: acc
                    .pda
                    .map(|pda| -> Result<t::IdlPda> {
                        Ok(t::IdlPda {
                            seeds: pda
                                .seeds
                                .into_iter()
                                .map(TryInto::try_into)
                                .collect::<Result<_>>()?,
                            program: pda.program_id.map(TryInto::try_into).transpose()?,
                        })
                    })
                    .transpose()
                    .unwrap_or_default(),
                relations: acc.relations,
            }),
            IV00::IdlAccountItem::IdlAccounts(accs) => Self::Composite(t::IdlInstructionAccounts {
                name: accs.name.to_snake_case(),
                accounts: accs.accounts.into_iter().map(Into::into).collect(),
            }),
        }
    }
}

impl TryFrom<IV00::IdlSeed> for t::IdlSeed {
    type Error = anyhow::Error;

    fn try_from(value: IV00::IdlSeed) -> Result<Self> {
        let seed = match value {
            IV00::IdlSeed::Account(seed) => Self::Account(t::IdlSeedAccount {
                account: seed.account,
                path: seed.path,
            }),
            IV00::IdlSeed::Arg(seed) => Self::Arg(t::IdlSeedArg { path: seed.path }),
            IV00::IdlSeed::Const(seed) => Self::Const(t::IdlSeedConst {
                value: match seed.ty {
                    IV00::IdlType::String => seed.value.to_string().as_bytes().into(),
                    _ => return Err(anyhow!("Const seed conversion not supported")),
                },
            }),
        };
        Ok(seed)
    }
}

pub fn parse_idl_with_compat(json_str: &str) -> Result<IdlV01, Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(json_str)?;
    let version = detect_idl_version(&json);

    match version {
        IdlVersion::V00 => {
            let value: IV00::Idl = serde_json::from_str(json_str).unwrap();
            let obj = IdlV01::try_from(value);
            Ok(obj.unwrap())
            //serde_json::from_value(value).map_err(Into::into)
            //Ok(serde_json::from_value(value).map_err(Into::<IdlV01>::into)?)
            //Ok(idl_v01)
        }
        IdlVersion::V01 => {
            let idl_v01: IdlV01 = serde_json::from_value(json)?;
            Ok(idl_v01)
        }
    }
}
#[pyfunction]
pub fn Convert_idl(json_str: &str) -> PyResult<String> {
    let idl = parse_idl_with_compat(json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()));
    Ok(serde_json::to_string_pretty(&idl.unwrap()).unwrap())
}
#[pyfunction]
pub fn Detect_idl(json_str: &str) -> PyResult<i32> {
    let json: Value = serde_json::from_str(json_str).unwrap();
    let version = detect_idl_version(&json);
    Ok(version as i32)
}
#[test]
fn test_detect_v00_format() {
    let json_str = r#"
        {
            "version": "0",
            "name": "MyProgram",
            "instructions": [
                {
                    "name": "Initialize",
                    "accounts": [
                        {
                            "name": "authority",
                            "isMut": true,
                            "isSigner": true
                        }
                    ],
                    "args": []
                }
            ]
        }
    "#;
    let res = parse_idl_with_compat(json_str);
    print!("{:?}", res.unwrap());
    //assert_eq!(version, IdlVersion::V00);
}
#[test]
fn test_detect_v00_format_02() {
    let json_str = r#"
        {
          "version": "2.106.0",
          "name": "drift",
          "instructions": [
            {
              "name": "initializeUser",
              "accounts": [
                {
                  "name": "user",
                  "isMut": true,
                  "isSigner": false
                }],
                "args": []
                }]
}
    "#;
    let res = parse_idl_with_compat(json_str);
    print!("{:?}", res.unwrap());
    //assert_eq!(version, IdlVersion::V00);
}
#[test]
fn test_detect_v00_format_03() {
    let json_str = include_str!("/Users/ttt/code/pysrc/anchorpy-dg/tests/idls/quarry_mine.json");
    let res = parse_idl_with_compat(json_str);
    print!("{:?}", res.unwrap());
    //assert_eq!(version, IdlVersion::V00);
}
#[test]
fn test_detect_v00_format_04() {
    let json_str = include_str!("/Users/ttt/code/pysrc/anchorpy-dg/tests/idls/composite.json");
    let res = parse_idl_with_compat(json_str);
    print!("{:?}", res.unwrap());
    //assert_eq!(version, IdlVersion::V00);
}
#[test]
fn test_detect_v00_format_05() {
    let json_str = include_str!("/Users/ttt/code/pysrc/anchorpy-dg/tests/idls/basic_0.json");
    let res = parse_idl_with_compat(json_str);
    print!("{:?}", res.unwrap());
    //assert_eq!(version, IdlVersion::V00);
}

/*/// Parse IDL from JSON string with automatic version detection and conversion
pub fn parse_idl_with_compat(json_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(json_str)?;
    let version = detect_idl_version(&json);

    match version {
        IdlVersion::V00 => {
            let idl_v00: IdlV00 = serde_json::from_value(json)?;
            let idl_v01 = convert_v00_to_v01(idl_v00)?;
            Ok(serde_json::to_string_pretty(&idl_v01).unwrap())
        }
        IdlVersion::V01 => {
            //let idl_v01: IdlV01 = serde_json::from_value(json)?;
            Ok(json_str.to_string())
        }
    }
}
#[pyfunction]
pub fn py_parse_idl_with_compat(json_str: &str) -> PyResult<String> {
    parse_idl_with_compat(json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_v00_format() {
        let v00_json = json!({
            "version": "0.1.0",
            "name": "test_program",
            "instructions": [
                {
                    "name": "initialize",
                    "accounts": [
                        {
                            "name": "authority",
                            "isMut": false,
                            "isSigner": true
                        }
                    ],
                    "args": []
                }
            ],
            "accounts": [],
            "types": []
        });

        assert_eq!(detect_idl_version(&v00_json), IdlVersion::V00);
    }

    #[test]
    fn test_detect_v01_format() {
        let v01_json = json!({
            "address": "11111111111111111111111111111111",
            "metadata": {
                "name": "test_program",
                "version": "0.1.0",
                "spec": "0.1.0"
            },
            "instructions": [],
            "accounts": [],
            "events": [],
            "errors": [],
            "types": [],
            "constants": []
        });

        assert_eq!(detect_idl_version(&v01_json), IdlVersion::V01);
    }

    #[test]
    fn test_convert_simple_v00_to_v01() {
        let v00_idl = IdlV00 {
            version: "0.1.0".to_string(),
            name: "test_program".to_string(),
            instructions: vec![],
            accounts: vec![],
            types: vec![],
            events: None,
            errors: None,
            metadata: None,
        };

        let result = convert_v00_to_v01(v00_idl).unwrap();
        assert_eq!(result.metadata.name, "test_program");
        assert_eq!(result.metadata.version, "0.1.0");
        assert_eq!(result.metadata.spec, "0.1.0");
    }
}
*/
