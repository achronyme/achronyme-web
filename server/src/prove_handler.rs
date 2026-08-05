//! Server-side prove handler.
//!
//! Thin wrapper around the shared [`prove_engine::ProveEngine`] — the same
//! compile→prove→verify pipeline the CLI drives. The server adds two
//! concerns the engine deliberately leaves out: an ephemeral cache
//! directory (systemd `ProtectHome` masks `$HOME`) and capturing the proof
//! artifacts into the HTTP response. Replaces a previously hand-copied
//! re-implementation that, among other drifts, proved over un-optimized
//! R1CS.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use akron::{ProveError, ProveHandler, ProveResult, VerifyHandler};
use memory::field::PrimeId;
use memory::FieldElement;
use prove_engine::{ProofEvent, ProveEngine, ProveObserver, ProveOptions};

use crate::playground_proving::playground_proving_policy;

/// Backend selection (re-exported so routes keep importing it from here).
pub use prove_engine::ProveBackend;

/// Captured proof artifact from a prove {} block execution.
#[derive(Clone, serde::Serialize)]
pub struct CapturedProof {
    pub name: String,
    pub constraints: usize,
    pub backend: String,
    pub proof_json: String,
    pub public_json: String,
    pub vkey_json: String,
}

/// Records the constraint / row count of the most recent proof. The count
/// is computed inside the engine and reported through the observer; it is
/// not part of the returned `ProveResult`, so the handler stashes it here
/// and reads it back when building the `CapturedProof`.
#[derive(Default)]
struct CountSink {
    last_count: RefCell<Option<usize>>,
}

impl ProveObserver for CountSink {
    fn on_proof_generated(&self, event: &ProofEvent) {
        *self.last_count.borrow_mut() = Some(event.count);
    }
}

pub struct ServerProveHandler {
    engine: ProveEngine,
    backend: ProveBackend,
    count_sink: Rc<CountSink>,
    /// Proof artifacts captured during VM execution.
    captured: RefCell<Vec<CapturedProof>>,
}

impl ServerProveHandler {
    pub fn new(backend: ProveBackend) -> Self {
        // /tmp/ach-server-cache: shared with routes/prove.rs and
        // routes/circuit.rs so warm Groth16 keys are reused across
        // /api/run + /api/prove + /api/circuit. $HOME is unsuitable
        // here — systemd's ProtectHome=true masks /home from the
        // service, and the cache is ephemeral anyway.
        let policy = playground_proving_policy();
        let count_sink = Rc::new(CountSink::default());
        let engine = ProveEngine::with_observer(
            ProveOptions {
                cache_dir: policy.cache_dir().to_path_buf(),
                backend,
                prime_id: PrimeId::Bn254,
                circuit_stats: false,
                key_source: policy.key_source().clone(),
            },
            Box::new(SharedSink(Rc::clone(&count_sink))),
        );
        Self {
            engine,
            backend,
            count_sink,
            captured: RefCell::new(Vec::new()),
        }
    }

    /// Drain captured proof artifacts.
    pub fn drain_captured(&self) -> Vec<CapturedProof> {
        self.captured.borrow_mut().drain(..).collect()
    }
}

/// Forwards observer callbacks to the shared `CountSink` (the engine owns
/// its observer, so the handler shares the sink via `Rc`).
struct SharedSink(Rc<CountSink>);

impl ProveObserver for SharedSink {
    fn on_proof_generated(&self, event: &ProofEvent) {
        self.0.on_proof_generated(event);
    }
}

impl ProveHandler for ServerProveHandler {
    fn execute_prove_ir(
        &self,
        prove_ir_bytes: &[u8],
        scope_values: &HashMap<String, FieldElement>,
    ) -> Result<ProveResult, ProveError> {
        let result = self.engine.execute_prove_ir(prove_ir_bytes, scope_values)?;

        if let ProveResult::Proof {
            ref proof_json,
            ref public_json,
            ref vkey_json,
        } = result
        {
            let constraints = self.count_sink.last_count.borrow_mut().take().unwrap_or(0);
            let backend = match self.backend {
                ProveBackend::R1cs => "r1cs",
                ProveBackend::Plonkish => "plonkish",
            };
            self.captured.borrow_mut().push(CapturedProof {
                name: "circuit".into(),
                constraints,
                backend: backend.into(),
                proof_json: proof_json.clone(),
                public_json: public_json.clone(),
                vkey_json: vkey_json.clone(),
            });
        }

        Ok(result)
    }
}

impl VerifyHandler for ServerProveHandler {
    fn verify_proof(&self, proof: &memory::ProofObject) -> Result<bool, String> {
        self.engine.verify(proof)
    }
}
