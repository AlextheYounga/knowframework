use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use know_core::{ConceptId, EntityId, RelationId};

const PREFIX: &str = "urn:know:";
pub(crate) const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
pub(crate) const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IriCodecError {
    InvalidScheme,
    InvalidKind { expected: &'static str, actual: String },
    InvalidHex,
    NonCanonicalHex,
    InvalidUtf8,
}

impl Display for IriCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::InvalidScheme => write!(formatter, "IRI is not a canonical urn:know IRI"),
            Self::InvalidKind { expected, actual } => {
                write!(formatter, "IRI kind {actual:?} cannot be decoded as {expected}")
            }
            Self::InvalidHex => write!(formatter, "IRI payload is not even-length hexadecimal"),
            Self::NonCanonicalHex => write!(formatter, "IRI payload must use lowercase hexadecimal"),
            Self::InvalidUtf8 => write!(formatter, "IRI payload is not valid UTF-8"),
        }
    }
}

impl Error for IriCodecError {}

#[must_use]
pub fn concept_iri(id: &ConceptId) -> String {
    encode("concept", &id.0)
}

#[must_use]
pub fn relation_iri(id: &RelationId) -> String {
    encode("relation", &id.0)
}

#[must_use]
pub fn entity_iri(id: &EntityId) -> String {
    encode("entity", &id.0)
}

/// Decode a canonical concept IRI.
///
/// # Errors
///
/// Returns [`IriCodecError`] when the IRI is malformed, noncanonical, or has another ID kind.
pub fn decode_concept_iri(iri: &str) -> Result<ConceptId, IriCodecError> {
    decode("concept", iri).map(ConceptId)
}

/// Decode a canonical relation IRI.
///
/// # Errors
///
/// Returns [`IriCodecError`] when the IRI is malformed, noncanonical, or has another ID kind.
pub fn decode_relation_iri(iri: &str) -> Result<RelationId, IriCodecError> {
    decode("relation", iri).map(RelationId)
}

/// Decode a canonical entity IRI.
///
/// # Errors
///
/// Returns [`IriCodecError`] when the IRI is malformed, noncanonical, or has another ID kind.
pub fn decode_entity_iri(iri: &str) -> Result<EntityId, IriCodecError> {
    decode("entity", iri).map(EntityId)
}

fn encode(kind: &str, id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut payload = String::with_capacity(id.len() * 2);
    for byte in id.bytes() {
        payload.push(char::from(HEX[usize::from(byte >> 4)]));
        payload.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    format!("{PREFIX}{kind}:{payload}")
}

fn decode(expected_kind: &'static str, iri: &str) -> Result<String, IriCodecError> {
    let Some(remainder) = iri.strip_prefix(PREFIX) else {
        return Err(IriCodecError::InvalidScheme);
    };
    let Some((kind, payload)) = remainder.split_once(':') else {
        return Err(IriCodecError::InvalidScheme);
    };
    if kind != expected_kind {
        return Err(IriCodecError::InvalidKind { expected: expected_kind, actual: kind.to_string() });
    }
    if payload.len() % 2 != 0 || !payload.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(IriCodecError::InvalidHex);
    }
    if payload.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(IriCodecError::NonCanonicalHex);
    }

    let bytes = payload
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = str::from_utf8(pair).map_err(|_| IriCodecError::InvalidHex)?;
            u8::from_str_radix(text, 16).map_err(|_| IriCodecError::InvalidHex)
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|_| IriCodecError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ids_round_trip_arbitrary_utf8() {
        let concept = ConceptId("módulo::猫".to_string());
        let relation = RelationId("módulo::has_猫".to_string());
        let entity = EntityId("módulo::Ada 🦀".to_string());

        assert_eq!(decode_concept_iri(&concept_iri(&concept)), Ok(concept));
        assert_eq!(decode_relation_iri(&relation_iri(&relation)), Ok(relation));
        assert_eq!(decode_entity_iri(&entity_iri(&entity)), Ok(entity));
        assert_eq!(decode_concept_iri("urn:know:concept:"), Ok(ConceptId(String::new())));
    }

    #[test]
    fn codec_rejects_noncanonical_malformed_and_cross_kind_iris() {
        assert!(matches!(decode_concept_iri("urn:know:concept:6A"), Err(IriCodecError::NonCanonicalHex)));
        assert!(matches!(decode_concept_iri("urn:know:concept:6"), Err(IriCodecError::InvalidHex)));
        assert!(matches!(decode_concept_iri("urn:know:concept:ff"), Err(IriCodecError::InvalidUtf8)));
        assert!(matches!(decode_concept_iri("urn:know:concept:6164:61"), Err(IriCodecError::InvalidHex)));
        assert!(matches!(decode_concept_iri("urn:know:entity:616461"), Err(IriCodecError::InvalidKind { .. })));
    }
}
