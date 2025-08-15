use crate::spec::idl_v0;
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
    V0,
    V1,
}
/// Detect the IDL version from JSON value
pub fn detect_idl_version(json: &Value) -> IdlVersion {
    // v0.1 has 'metadata' field with 'spec' inside
    if let Some(metadata) = json.get("metadata") {
        if metadata.get("spec").is_some() {
            return IdlVersion::V0;
        }
    }

    // v0.1 has 'address' field at root level
    //if json.get("address").is_some() {
    //    return IdlVersion::V01;
    //}

    // v0.0 has 'version' and 'name' at root level without 'metadata'
    if json.get("version").is_some() && json.get("name").is_some() && json.get("metadata").is_none()
    {
        return IdlVersion::V0;
    }

    // Check for v0.0 specific account structure (is_mut, is_signer)
    if let Some(instructions) = json.get("instructions") {
        if let Some(instruction) = instructions.as_array().and_then(|arr| arr.first()) {
            if instruction.get("discriminator").is_none() {
                return IdlVersion::V0;
            }
            if let Some(accounts) = instruction.get("accounts") {
                if let Some(account) = accounts.as_array().and_then(|arr| arr.first()) {
                    if account.get("is_mut").is_some()
                        || account.get("is_signer").is_some() | account.get("is_Mut").is_some()
                        || account.get("is_Signer").is_some() | account.get("isMut").is_some()
                        || account.get("isSigner").is_some()
                    {
                        return IdlVersion::V0;
                    }
                }
            }
        }
    }

    // Default to v0.1 if uncertain
    IdlVersion::V1
}

impl TryFrom<idl_v0::Idl> for t::Idl {
    type Error = anyhow::Error;

    fn try_from(idl: idl_v0::Idl) -> Result<Self> {
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

impl From<idl_v0::IdlInstruction> for t::IdlInstruction {
    fn from(value: idl_v0::IdlInstruction) -> Self {
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

impl From<idl_v0::IdlTypeDefinition> for t::IdlAccount {
    fn from(value: idl_v0::IdlTypeDefinition) -> Self {
        Self {
            discriminator: get_disc("account", &value.name),
            name: value.name,
        }
    }
}

impl From<idl_v0::IdlEvent> for t::IdlEvent {
    fn from(value: idl_v0::IdlEvent) -> Self {
        Self {
            discriminator: get_disc("event", &value.name),
            name: value.name,
        }
    }
}

impl From<idl_v0::IdlErrorCode> for t::IdlErrorCode {
    fn from(value: idl_v0::IdlErrorCode) -> Self {
        Self {
            name: value.name,
            code: value.code,
            msg: value.msg,
        }
    }
}

impl From<idl_v0::IdlConst> for t::IdlConst {
    fn from(value: idl_v0::IdlConst) -> Self {
        Self {
            name: value.name,
            docs: Default::default(),
            ty: value.ty.into(),
            value: value.value,
        }
    }
}

impl From<idl_v0::IdlDefinedTypeArg> for t::IdlGenericArg {
    fn from(value: idl_v0::IdlDefinedTypeArg) -> Self {
        match value {
            idl_v0::IdlDefinedTypeArg::Type(ty) => Self::Type { ty: ty.into() },
            idl_v0::IdlDefinedTypeArg::Value(value) => Self::Const { value },
            idl_v0::IdlDefinedTypeArg::Generic(generic) => Self::Type {
                ty: t::IdlType::Generic(generic),
            },
        }
    }
}

impl From<idl_v0::IdlTypeDefinition> for t::IdlTypeDef {
    fn from(value: idl_v0::IdlTypeDefinition) -> Self {
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

impl From<idl_v0::IdlEvent> for t::IdlTypeDef {
    fn from(value: idl_v0::IdlEvent) -> Self {
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

impl From<idl_v0::IdlTypeDefinitionTy> for t::IdlTypeDefTy {
    fn from(value: idl_v0::IdlTypeDefinitionTy) -> Self {
        match value {
            idl_v0::IdlTypeDefinitionTy::Struct { fields } => Self::Struct {
                fields: if fields.is_empty() {
                    None
                } else {
                    Some(t::IdlDefinedFields::Named(
                        fields.into_iter().map(Into::into).collect(),
                    ))
                },
            },
            idl_v0::IdlTypeDefinitionTy::Enum { variants } => Self::Enum {
                variants: variants
                    .into_iter()
                    .map(|variant| t::IdlEnumVariant {
                        name: variant.name,
                        fields: variant.fields.map(|fields| match fields {
                            idl_v0::EnumFields::Named(fields) => t::IdlDefinedFields::Named(
                                fields.into_iter().map(Into::into).collect(),
                            ),
                            idl_v0::EnumFields::Tuple(tys) => t::IdlDefinedFields::Tuple(
                                tys.into_iter().map(Into::into).collect(),
                            ),
                        }),
                    })
                    .collect(),
            },
            idl_v0::IdlTypeDefinitionTy::Alias { value } => Self::Type {
                alias: value.into(),
            },
        }
    }
}

impl From<idl_v0::IdlField> for t::IdlField {
    fn from(value: idl_v0::IdlField) -> Self {
        Self {
            name: value.name.to_snake_case(),
            docs: value.docs.unwrap_or_default(),
            ty: value.ty.into(),
        }
    }
}

impl From<idl_v0::IdlType> for t::IdlType {
    fn from(value: idl_v0::IdlType) -> Self {
        match value {
            idl_v0::IdlType::PublicKey => t::IdlType::Pubkey,
            idl_v0::IdlType::Defined(name) => t::IdlType::Defined {
                name,
                generics: Default::default(),
            },
            idl_v0::IdlType::DefinedWithTypeArgs { name, args } => t::IdlType::Defined {
                name,
                generics: args.into_iter().map(Into::into).collect(),
            },
            idl_v0::IdlType::Option(ty) => t::IdlType::Option(ty.into()),
            idl_v0::IdlType::Vec(ty) => t::IdlType::Vec(ty.into()),
            idl_v0::IdlType::Array(ty, len) => {
                t::IdlType::Array(ty.into(), t::IdlArrayLen::Value(len))
            }
            idl_v0::IdlType::GenericLenArray(ty, generic) => {
                t::IdlType::Array(ty.into(), t::IdlArrayLen::Generic(generic))
            }
            _ => serde_json::to_value(value)
                .and_then(serde_json::from_value)
                .unwrap(),
        }
    }
}

impl From<Box<idl_v0::IdlType>> for Box<t::IdlType> {
    fn from(value: Box<idl_v0::IdlType>) -> Self {
        Box::new((*value).into())
    }
}

impl From<idl_v0::IdlAccountItem> for t::IdlInstructionAccountItem {
    fn from(value: idl_v0::IdlAccountItem) -> Self {
        match value {
            idl_v0::IdlAccountItem::IdlAccount(acc) => Self::Single(t::IdlInstructionAccount {
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
            idl_v0::IdlAccountItem::IdlAccounts(accs) => {
                Self::Composite(t::IdlInstructionAccounts {
                    name: accs.name.to_snake_case(),
                    accounts: accs.accounts.into_iter().map(Into::into).collect(),
                })
            }
        }
    }
}

impl TryFrom<idl_v0::IdlSeed> for t::IdlSeed {
    type Error = anyhow::Error;

    fn try_from(value: idl_v0::IdlSeed) -> Result<Self> {
        let seed = match value {
            idl_v0::IdlSeed::Account(seed) => Self::Account(t::IdlSeedAccount {
                account: seed.account,
                path: seed.path,
            }),
            idl_v0::IdlSeed::Arg(seed) => Self::Arg(t::IdlSeedArg { path: seed.path }),
            idl_v0::IdlSeed::Const(seed) => Self::Const(t::IdlSeedConst {
                value: match seed.ty {
                    idl_v0::IdlType::String => seed.value.to_string().as_bytes().into(),
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
        IdlVersion::V0 => {
            let value: idl_v0::Idl = serde_json::from_str(json_str).unwrap();
            let obj = IdlV01::try_from(value);
            Ok(obj.unwrap())
        }
        IdlVersion::V1 => {
            let idl_v01: IdlV01 = serde_json::from_value(json)?;
            Ok(idl_v01)
        }
    }
}
#[pyfunction]
pub fn convert_idl(json_str: &str) -> PyResult<String> {
    let idl = parse_idl_with_compat(json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()));
    Ok(serde_json::to_string_pretty(&idl.unwrap()).unwrap())
}
#[pyfunction]
pub fn detect_idl(json_str: &str) -> PyResult<i32> {
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
