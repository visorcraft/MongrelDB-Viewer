//! Dense embedding backends for ANN install + semantic search.
//!
//! Default local model: `all-MiniLM-L6-v2` (384 dimensions).
//! Users may also point at any OpenAI-compatible embeddings endpoint.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{EmbedResponse, LocalModelInfo, ProviderInfo};

pub const DEFAULT_PROVIDER_ID: &str = "viewer-minilm";
pub const DEFAULT_MODEL_ID: &str = "all-MiniLM-L6-v2";
pub const DEFAULT_MODEL_VERSION: &str = "1";
pub const DEFAULT_DIM: u32 = 384;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEmbedConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub dimension: u32,
    pub provider_id: String,
}

#[derive(Clone)]
pub struct EmbeddingHub {
    inner: Arc<Mutex<HubInner>>,
}

struct HubInner {
    local_ready: bool,
    local_error: Option<String>,
    #[cfg(feature = "local-embeddings")]
    local: Option<LocalMiniLm>,
    remote: Option<RemoteEmbedConfig>,
}

impl Default for EmbeddingHub {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HubInner {
                local_ready: false,
                local_error: None,
                #[cfg(feature = "local-embeddings")]
                local: None,
                remote: None,
            })),
        }
    }
}

impl EmbeddingHub {
    pub fn available_models() -> Vec<LocalModelInfo> {
        vec![
            LocalModelInfo {
                id: DEFAULT_MODEL_ID.into(),
                label: "all-MiniLM-L6-v2 (default, 384-d)".into(),
                dimension: 384,
                default: true,
            },
            LocalModelInfo {
                id: "all-MiniLM-L12-v2".into(),
                label: "all-MiniLM-L12-v2 (384-d)".into(),
                dimension: 384,
                default: false,
            },
            LocalModelInfo {
                id: "bge-small-en-v1.5".into(),
                label: "BGE small en v1.5 (384-d)".into(),
                dimension: 384,
                default: false,
            },
            LocalModelInfo {
                id: "remote-openai-compatible".into(),
                label: "Remote OpenAI-compatible embeddings".into(),
                dimension: 0,
                default: false,
            },
        ]
    }

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        let g = self.inner.lock();
        let mut out = Vec::new();
        if g.local_ready {
            out.push(ProviderInfo {
                provider_id: DEFAULT_PROVIDER_ID.into(),
                model_id: DEFAULT_MODEL_ID.into(),
                model_version: DEFAULT_MODEL_VERSION.into(),
                dimension: DEFAULT_DIM,
                health: "ready".into(),
                backend: "local-fastembed".into(),
            });
        } else if let Some(err) = &g.local_error {
            out.push(ProviderInfo {
                provider_id: DEFAULT_PROVIDER_ID.into(),
                model_id: DEFAULT_MODEL_ID.into(),
                model_version: DEFAULT_MODEL_VERSION.into(),
                dimension: DEFAULT_DIM,
                health: format!("unavailable: {err}"),
                backend: "local-fastembed".into(),
            });
        }
        if let Some(remote) = &g.remote {
            out.push(ProviderInfo {
                provider_id: remote.provider_id.clone(),
                model_id: remote.model.clone(),
                model_version: "remote".into(),
                dimension: remote.dimension,
                health: "ready".into(),
                backend: "openai-compatible".into(),
            });
        }
        out
    }

    /// Lazily install / load the default local model.
    pub fn ensure_local_default(&self) -> AppResult<()> {
        #[cfg(feature = "local-embeddings")]
        {
            let mut g = self.inner.lock();
            if g.local.is_some() {
                g.local_ready = true;
                return Ok(());
            }
            match LocalMiniLm::load(DEFAULT_MODEL_ID) {
                Ok(model) => {
                    g.local = Some(model);
                    g.local_ready = true;
                    g.local_error = None;
                    Ok(())
                }
                Err(e) => {
                    g.local_ready = false;
                    g.local_error = Some(e.to_string());
                    Err(e)
                }
            }
        }
        #[cfg(not(feature = "local-embeddings"))]
        {
            Err(AppError::Embedding(
                "local embeddings were disabled at build time; configure a remote OpenAI-compatible embeddings endpoint".into(),
            ))
        }
    }

    pub fn configure_remote(&self, cfg: RemoteEmbedConfig) {
        self.inner.lock().remote = Some(cfg);
    }

    pub fn embed(&self, texts: &[String], provider_id: Option<&str>) -> AppResult<EmbedResponse> {
        if texts.is_empty() {
            return Err(AppError::Embedding("no texts provided".into()));
        }
        let want = provider_id.unwrap_or(DEFAULT_PROVIDER_ID);

        // Prefer remote when explicitly requested or when it matches configured id.
        {
            let g = self.inner.lock();
            if let Some(remote) = &g.remote {
                if want == remote.provider_id || want == "remote" || want == "remote-openai-compatible"
                {
                    let remote = remote.clone();
                    drop(g);
                    return self.embed_remote(&remote, texts);
                }
            }
        }

        // Local default path
        self.ensure_local_default()?;
        #[cfg(feature = "local-embeddings")]
        {
            let g = self.inner.lock();
            let local = g
                .local
                .as_ref()
                .ok_or_else(|| AppError::Embedding("local model not loaded".into()))?;
            let vectors = local.embed(texts)?;
            let dim = vectors.first().map(|v| v.len() as u32).unwrap_or(DEFAULT_DIM);
            Ok(EmbedResponse {
                vectors,
                dimension: dim,
                provider_id: DEFAULT_PROVIDER_ID.into(),
                model_id: DEFAULT_MODEL_ID.into(),
            })
        }
        #[cfg(not(feature = "local-embeddings"))]
        {
            let _ = want;
            Err(AppError::Embedding(
                "no embedding backend available".into(),
            ))
        }
    }

    /// Register the active Viewer embedding backend on a Direct `Database` so
    /// engine-native surfaces (`retrieve_text`, semantic identity binding) can
    /// resolve the same MiniLM/remote provider the Viewer uses for install.
    pub fn register_on_database(&self, db: &mongreldb_core::Database) -> AppResult<()> {
        let providers = self.list_providers();
        let ready = providers
            .into_iter()
            .find(|p| p.health == "ready")
            .ok_or_else(|| {
                AppError::Embedding(
                    "no ready embedding provider — load MiniLM or configure a remote endpoint"
                        .into(),
                )
            })?;

        let provider: Arc<dyn mongreldb_core::EmbeddingProvider> =
            Arc::new(ViewerEmbeddingProvider {
                hub: self.clone(),
                provider_id: ready.provider_id.clone(),
                model_id: ready.model_id.clone(),
                model_version: ready.model_version.clone(),
                dimension: ready.dimension,
            });

        match db.embedding_providers().register_new(provider) {
            Ok(_) => Ok(()),
            Err(mongreldb_core::EmbeddingError::ProviderAlreadyRegistered(_)) => Ok(()),
            Err(e) => Err(AppError::Embedding(e.to_string())),
        }
    }

    /// Catalog source metadata for embedding columns written by this Viewer.
    pub fn configured_source(&self, provider_id: Option<&str>) -> mongreldb_core::EmbeddingSource {
        let want = provider_id.unwrap_or(DEFAULT_PROVIDER_ID);
        let g = self.inner.lock();
        if let Some(remote) = &g.remote {
            if want == remote.provider_id || want == "remote" || want == "remote-openai-compatible" {
                return mongreldb_core::EmbeddingSource::ConfiguredModel {
                    provider_id: remote.provider_id.clone(),
                    model_id: remote.model.clone(),
                    model_version: "remote".into(),
                };
            }
        }
        mongreldb_core::EmbeddingSource::ConfiguredModel {
            provider_id: DEFAULT_PROVIDER_ID.into(),
            model_id: DEFAULT_MODEL_ID.into(),
            model_version: DEFAULT_MODEL_VERSION.into(),
        }
    }

    fn embed_remote(&self, cfg: &RemoteEmbedConfig, texts: &[String]) -> AppResult<EmbedResponse> {
        let base = cfg.base_url.trim_end_matches('/');
        let url = if base.ends_with("/embeddings") {
            base.to_string()
        } else if base.ends_with("/v1") {
            format!("{base}/embeddings")
        } else {
            format!("{base}/v1/embeddings")
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::Http(e.to_string()))?;

        let body = serde_json::json!({
            "model": cfg.model,
            "input": texts,
        });

        let mut req = client.post(&url).json(&body);
        if !cfg.api_key.is_empty() {
            req = req.bearer_auth(&cfg.api_key);
        }

        let resp = req
            .send()
            .map_err(|e| AppError::Http(e.to_string()))?
            .error_for_status()
            .map_err(|e| AppError::Http(e.to_string()))?;

        let json: serde_json::Value = resp.json().map_err(|e| AppError::Http(e.to_string()))?;
        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| AppError::Embedding("remote embeddings response missing data[]".into()))?;

        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let emb = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| AppError::Embedding("missing embedding vector".into()))?;
            let vec: Vec<f32> = emb
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            vectors.push(vec);
        }

        let dim = vectors
            .first()
            .map(|v| v.len() as u32)
            .unwrap_or(cfg.dimension);

        Ok(EmbedResponse {
            vectors,
            dimension: dim,
            provider_id: cfg.provider_id.clone(),
            model_id: cfg.model.clone(),
        })
    }
}

/// Bridges [`EmbeddingHub`] into MongrelDB's process-local provider registry
/// (0.64 semantic identity + `retrieve_text`).
struct ViewerEmbeddingProvider {
    hub: EmbeddingHub,
    provider_id: String,
    model_id: String,
    model_version: String,
    dimension: u32,
}

impl mongreldb_core::EmbeddingProvider for ViewerEmbeddingProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }
    fn model_id(&self) -> &str {
        &self.model_id
    }
    fn model_version(&self) -> &str {
        &self.model_version
    }
    fn dimension(&self) -> u32 {
        self.dimension
    }
    fn normalization(&self) -> mongreldb_core::EmbeddingNormalization {
        mongreldb_core::EmbeddingNormalization::None
    }
    fn preprocessing_version(&self) -> &str {
        "viewer-1"
    }
    fn embed(
        &self,
        request: mongreldb_core::EmbeddingRequest<'_>,
    ) -> Result<mongreldb_core::EmbeddingResponse, mongreldb_core::EmbeddingError> {
        let texts: Vec<String> = request.texts.iter().map(|s| (*s).to_string()).collect();
        match self.hub.embed(&texts, Some(&self.provider_id)) {
            Ok(r) => Ok(mongreldb_core::EmbeddingResponse { vectors: r.vectors }),
            Err(e) => Err(mongreldb_core::EmbeddingError::ProviderFailed {
                provider: self.provider_id.clone(),
                message: e.to_string(),
            }),
        }
    }
}

#[cfg(feature = "local-embeddings")]
struct LocalMiniLm {
    model: Mutex<fastembed::TextEmbedding>,
    model_id: String,
}

#[cfg(feature = "local-embeddings")]
impl LocalMiniLm {
    fn load(model_id: &str) -> AppResult<Self> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

        let embedding_model = match model_id {
            "all-MiniLM-L12-v2" => EmbeddingModel::AllMiniLML12V2,
            "bge-small-en-v1.5" => EmbeddingModel::BGESmallENV15,
            _ => EmbeddingModel::AllMiniLML6V2,
        };

        let cache = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("mongreldb-viewer")
            .join("models");
        std::fs::create_dir_all(&cache).ok();

        let model = TextEmbedding::try_new(
            InitOptions::new(embedding_model)
                .with_show_download_progress(true)
                .with_cache_dir(cache),
        )
        .map_err(|e| AppError::Embedding(format!("failed to load {model_id}: {e}")))?;

        Ok(Self {
            model: Mutex::new(model),
            model_id: model_id.to_string(),
        })
    }

    fn embed(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let mut model = self.model.lock();
        model
            .embed(refs, None)
            .map_err(|e| AppError::Embedding(format!("{} embed failed: {e}", self.model_id)))
    }
}
