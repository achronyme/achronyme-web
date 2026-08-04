use std::path::PathBuf;

use proving::groth16::ProvingKeySource;

pub(crate) const PLAYGROUND_PROOF_TRUST_NOTICE: &str =
    "Playground proofs use an insecure local development setup and are not production-trusted proofs.";

#[derive(Clone, Debug)]
pub(crate) struct PlaygroundProvingPolicy {
    cache_dir: PathBuf,
    key_source: ProvingKeySource,
}

impl PlaygroundProvingPolicy {
    pub(crate) fn cache_dir(&self) -> &std::path::Path {
        &self.cache_dir
    }

    pub(crate) fn key_source(&self) -> &ProvingKeySource {
        &self.key_source
    }
}

pub(crate) fn playground_proving_policy() -> PlaygroundProvingPolicy {
    PlaygroundProvingPolicy {
        cache_dir: std::env::temp_dir().join("ach-server-cache"),
        key_source: playground_key_source(),
    }
}

fn playground_key_source() -> ProvingKeySource {
    ProvingKeySource::InsecureLocal
}

pub(crate) fn require_development_key_source(key_source: &ProvingKeySource) -> Result<(), String> {
    if matches!(key_source, ProvingKeySource::InsecureLocal) {
        Ok(())
    } else {
        Err(PLAYGROUND_PROOF_TRUST_NOTICE.to_string())
    }
}

#[cfg(test)]
mod tests {
    use proving::groth16::ProvingKeySource;

    use super::{playground_proving_policy, require_development_key_source};

    #[test]
    fn playground_proving_uses_explicit_development_setup() {
        assert!(matches!(
            playground_proving_policy().key_source,
            ProvingKeySource::InsecureLocal
        ));
    }

    #[test]
    fn playground_rejects_other_key_sources_for_development_proofs() {
        let policy = playground_proving_policy();
        assert!(require_development_key_source(policy.key_source()).is_ok());
        assert!(require_development_key_source(&ProvingKeySource::DenyInsecureSetup).is_err());
    }
}
