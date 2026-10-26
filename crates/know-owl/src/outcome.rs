use know_reasoner::{ReasoningOutcome, ResourceLimit, ResourceLimitKind};

pub(crate) fn normalize<T>(result: Result<T, owl_dl_reasoner::ReasonError>) -> ReasoningOutcome<T> {
    match result {
        Ok(value) => ReasoningOutcome::Complete(value),
        Err(owl_dl_reasoner::ReasonError::NoVerdict) => {
            ReasoningOutcome::ResourceLimit(ResourceLimit { kind: ResourceLimitKind::Iterations })
        }
        Err(owl_dl_reasoner::ReasonError::Conversion(error)) => ReasoningOutcome::InternalError(format!(
            "rustdl could not convert an adapter-generated query probe: {error}"
        )),
        Err(owl_dl_reasoner::ReasonError::RoleChainUnsupported) => {
            ReasoningOutcome::InternalError("rustdl rejected an adapter-generated query probe role chain".to_string())
        }
        Err(error) => ReasoningOutcome::InternalError(format!("rustdl execution failed: {error}")),
    }
}

pub(crate) fn init_error(error: owl_dl_reasoner::ReasonError) -> crate::RustdlInitError {
    match error {
        owl_dl_reasoner::ReasonError::Conversion(error) => crate::RustdlInitError::Unsupported(error.to_string()),
        owl_dl_reasoner::ReasonError::RoleChainUnsupported => {
            crate::RustdlInitError::Unsupported("rustdl cannot represent this OWL role chain".to_string())
        }
        error => crate::RustdlInitError::InternalError(error.to_string()),
    }
}

pub(crate) fn propagate<T, U>(outcome: ReasoningOutcome<T>) -> ReasoningOutcome<U> {
    match outcome {
        ReasoningOutcome::Complete(_) => {
            ReasoningOutcome::InternalError("unexpected completed backend probe".to_string())
        }
        ReasoningOutcome::Unsupported(feature) => ReasoningOutcome::Unsupported(feature),
        ReasoningOutcome::ResourceLimit(limit) => ReasoningOutcome::ResourceLimit(limit),
        ReasoningOutcome::InternalError(error) => ReasoningOutcome::InternalError(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use know_reasoner::UnsupportedFeature;

    #[test]
    fn normalize_no_verdict_as_resource_limit() {
        assert!(matches!(
            normalize::<bool>(Err(owl_dl_reasoner::ReasonError::NoVerdict)),
            ReasoningOutcome::ResourceLimit(_)
        ));
    }

    #[test]
    fn normalize_backend_failures_as_internal_errors() {
        assert!(matches!(
            normalize::<bool>(Err(owl_dl_reasoner::ReasonError::UnknownClass("probe".to_string()))),
            ReasoningOutcome::InternalError(_)
        ));
        assert!(matches!(
            normalize::<bool>(Err(owl_dl_reasoner::ReasonError::Conversion(
                owl_dl_core::ConversionError::AnonymousIndividual,
            ))),
            ReasoningOutcome::InternalError(_)
        ));
        assert!(matches!(
            normalize::<bool>(Err(owl_dl_reasoner::ReasonError::RoleChainUnsupported)),
            ReasoningOutcome::InternalError(_)
        ));
    }

    #[test]
    fn initialization_failures_are_internal_errors() {
        assert!(matches!(
            init_error(owl_dl_reasoner::ReasonError::NoVerdict),
            crate::RustdlInitError::InternalError(_)
        ));
    }

    #[test]
    fn propagate_preserves_non_complete_outcomes() {
        assert!(matches!(
            propagate::<bool, bool>(ReasoningOutcome::Unsupported(UnsupportedFeature {
                feature: "test".to_string(),
                advice: None,
            })),
            ReasoningOutcome::Unsupported(_)
        ));
        assert!(matches!(
            propagate::<bool, bool>(ReasoningOutcome::ResourceLimit(ResourceLimit { kind: ResourceLimitKind::Time })),
            ReasoningOutcome::ResourceLimit(_)
        ));
        assert!(matches!(
            propagate::<bool, bool>(ReasoningOutcome::InternalError("test".to_string())),
            ReasoningOutcome::InternalError(_)
        ));
    }
}
