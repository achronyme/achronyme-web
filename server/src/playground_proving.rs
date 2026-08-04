#[cfg(test)]
mod tests {
    use proving::groth16::ProvingKeySource;

    use super::playground_key_source;

    #[test]
    fn playground_proving_uses_explicit_development_setup() {
        assert!(matches!(
            playground_key_source(),
            ProvingKeySource::InsecureLocal
        ));
    }
}
