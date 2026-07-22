use mongreldb_core::schema::{AlterColumn, ColumnFlags, IndexKind, TypeId};
use mongreldb_core::{EmbeddingSource, TextSearchOptions, Value};

use crate::db::session::DbSession;
use crate::db::sql::run_sql;
use crate::embeddings::{EmbeddingHub, DEFAULT_DIM, DEFAULT_PROVIDER_ID};
use crate::error::{AppError, AppResult};
use crate::models::{
    InstallAnnRequest, InstallAnnResult, SearchProvenance, SemanticSearchRequest, SqlRequest,
    SqlResult,
};

/// Install an ANN surface on a table.
///
/// - Ensures an Embedding column (default dim 384)
/// - Creates an Ann secondary index via SQL (default **hnsw × dense** f32 cosine)
/// - Optionally backfills vectors from a text column using the selected provider
///
/// MongrelDB 0.63+: algorithm (`hnsw` / `diskann` / `ivf`) is independent of
/// quantization (`dense` / `binary_sign` / `product`). Supported pairs match
/// the engine: `hnsw × {binary_sign, dense, product}`, `diskann × dense`,
/// `ivf × dense`.
pub async fn install_dense_ann(
    db: &DbSession,
    embeddings: &EmbeddingHub,
    req: InstallAnnRequest,
) -> AppResult<InstallAnnResult> {
    let table = req.table.trim().to_string();
    if table.is_empty() {
        return Err(AppError::msg("table name required"));
    }
    let emb_col = req
        .embedding_column
        .as_deref()
        .unwrap_or("embedding")
        .to_string();
    let dim = req.dimension.unwrap_or(DEFAULT_DIM);
    if dim == 0 || dim > 4096 {
        return Err(AppError::msg("dimension must be between 1 and 4096"));
    }
    let quantization = normalize_quantization(req.quantization.as_deref())?;
    let algorithm = normalize_algorithm(req.algorithm.as_deref())?;
    validate_ann_pair(algorithm, quantization)?;
    if quantization == "product" {
        let nsub = req.product_num_subvectors.ok_or_else(|| {
            AppError::msg(
                "product quantization requires productNumSubvectors (must divide dimension)",
            )
        })?;
        if nsub == 0 || !dim.is_multiple_of(u32::from(nsub)) {
            return Err(AppError::msg(format!(
                "productNumSubvectors ({nsub}) must be > 0 and evenly divide dimension ({dim})"
            )));
        }
        let bits = req.product_bits.unwrap_or(8);
        if bits != 8 {
            return Err(AppError::msg(
                "product bitsPerSubvector must be 8 (only value supported by the engine)",
            ));
        }
    }
    let quant_label = quantization_label(quantization);
    let algo_label = algorithm_label(algorithm);
    let rebuild = req.rebuild.unwrap_or(false);
    let mut index_name = req
        .index_name
        .clone()
        .unwrap_or_else(|| format!("{table}_{emb_col}_ann"));
    let provider_id = req
        .provider_id
        .clone()
        .unwrap_or_else(|| DEFAULT_PROVIDER_ID.to_string());

    if req.source_text_column.is_some() && provider_id == DEFAULT_PROVIDER_ID {
        embeddings.ensure_local_default()?;
    }
    // Register on the engine so 0.64 retrieve_text / semantic identity can resolve.
    if req.source_text_column.is_some() {
        let _ = embeddings.register_on_database(&db.database);
    }

    // Mutate schema synchronously; no awaits while table guards are live.
    // ANN presence is durable in the table schema - survives close/reopen.
    let mut has_ann = ensure_embedding_column_and_check_ann(db, &table, &emb_col, dim)?;
    // Stamp configured_model source so native retrieve_text can resolve the provider
    // even before the ANN graph binds a live semantic identity.
    if req.source_text_column.is_some() {
        let _ =
            stamp_embedding_source(db, &table, &emb_col, embeddings, Some(provider_id.as_str()));
    }
    let text_col = req
        .source_text_column
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let mut rebuilt = false;
    if rebuild && has_ann {
        let existing_name =
            existing_ann_index_name(db, &table, &emb_col).unwrap_or_else(|| index_name.clone());
        index_name = existing_name.clone();
        let drop_sql = format!("DROP INDEX {existing_name} ON {table}");
        db.session
            .run(&drop_sql)
            .await
            .map_err(|e| AppError::sql(format!("DROP INDEX failed: {e}")))?;
        has_ann = false;
        rebuilt = true;
    }

    // Already fully installed and no re-embed / rebuild requested - do nothing.
    if has_ann && text_col.is_none() {
        let existing = existing_ann_quantization(db, &table, &emb_col).unwrap_or(quantization);
        let existing_algo = existing_ann_algorithm(db, &table, &emb_col).unwrap_or(algorithm);
        return Ok(InstallAnnResult {
            table: table.clone(),
            embedding_column: emb_col,
            dimension: dim,
            index_name,
            rows_embedded: 0,
            already_ready: true,
            quantization: existing.to_string(),
            algorithm: existing_algo.to_string(),
            rebuilt: false,
            message: format!(
                "{} {} ANN already active on {table} ({dim}-d, quant={existing}). Stored with the database - no install needed. Use rebuild to change algorithm/quantization.",
                algorithm_label(existing_algo),
                quantization_label(existing),
            ),
        });
    }

    if !has_ann {
        let sql =
            build_create_ann_sql(&index_name, &table, &emb_col, algorithm, quantization, &req)?;
        db.session
            .run(&sql)
            .await
            .map_err(|e| AppError::sql(format!("CREATE INDEX failed: {e}")))?;
    }

    let mut rows_embedded = 0u64;
    if let Some(text_col) = text_col {
        rows_embedded = backfill_embeddings(
            db,
            embeddings,
            &table,
            &text_col,
            &emb_col,
            dim,
            Some(provider_id.as_str()),
            req.backfill_limit.unwrap_or(10_000),
        )
        .await?;
    }

    let active_quant = if has_ann && !rebuilt {
        existing_ann_quantization(db, &table, &emb_col).unwrap_or(quantization)
    } else {
        quantization
    };
    let active_algo = if has_ann && !rebuilt {
        existing_ann_algorithm(db, &table, &emb_col).unwrap_or(algorithm)
    } else {
        algorithm
    };

    let message = if rebuilt {
        format!(
            "Rebuilt {algo_label} {quant_label} ANN on {table} ({dim}-d, algorithm={algorithm}, quantization={quantization}). Provider={provider_id}. Rows embedded={rows_embedded}."
        )
    } else if has_ann {
        format!(
            "{} {} ANN already active on {table} ({dim}-d, {active_quant}). Re-embedded {rows_embedded} rows via {provider_id}.",
            algorithm_label(active_algo),
            quantization_label(active_quant)
        )
    } else {
        format!(
            "{algo_label} {quant_label} ANN ready on {table} ({dim}-d, algorithm={algorithm}, quantization={quantization}). Provider={provider_id}. Rows embedded={rows_embedded}."
        )
    };

    Ok(InstallAnnResult {
        table: table.clone(),
        embedding_column: emb_col,
        dimension: dim,
        index_name,
        rows_embedded,
        already_ready: has_ann && !rebuilt,
        quantization: active_quant.to_string(),
        algorithm: active_algo.to_string(),
        rebuilt,
        message,
    })
}

/// Build `CREATE INDEX … USING ann … WITH (…)` for the requested backend pair.
fn build_create_ann_sql(
    index_name: &str,
    table: &str,
    emb_col: &str,
    algorithm: &str,
    quantization: &str,
    req: &InstallAnnRequest,
) -> AppResult<String> {
    let m = req.m.unwrap_or(16);
    let efc = req.ef_construction.unwrap_or(64);
    let efs = req.ef_search.unwrap_or(64);
    let mut with = vec![
        format!("m = {m}"),
        format!("ef_construction = {efc}"),
        format!("ef_search = {efs}"),
        format!("algorithm = '{algorithm}'"),
        format!("quantization = '{quantization}'"),
    ];
    match algorithm {
        "diskann" => {
            if let Some(r) = req.diskann_r {
                with.push(format!("diskann_r = {r}"));
            }
            if let Some(l) = req.diskann_l {
                with.push(format!("diskann_l = {l}"));
            }
            if let Some(b) = req.diskann_beam_width {
                with.push(format!("beam_width = {b}"));
            }
        }
        "ivf" => {
            if let Some(n) = req.ivf_nlist {
                with.push(format!("nlist = {n}"));
            }
            if let Some(n) = req.ivf_nprobe {
                with.push(format!("nprobe = {n}"));
            }
        }
        _ => {}
    }
    if quantization == "product" {
        let nsub = req
            .product_num_subvectors
            .ok_or_else(|| AppError::msg("product quantization requires productNumSubvectors"))?;
        let bits = req.product_bits.unwrap_or(8);
        with.push(format!("num_subvectors = {nsub}"));
        with.push(format!("bits_per_subvector = {bits}"));
    }
    Ok(format!(
        "CREATE INDEX {index_name} ON {table} USING ann ({emb_col}) WITH ({})",
        with.join(", ")
    ))
}

/// Normalize user/API quantization to engine SQL literals.
fn normalize_quantization(raw: Option<&str>) -> AppResult<&'static str> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok("dense"),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "dense" => Ok("dense"),
            "binary_sign" | "binary-sign" | "binary" | "hamming" => Ok("binary_sign"),
            "product" | "pq" => Ok("product"),
            other => Err(AppError::msg(format!(
                "quantization must be 'dense', 'binary_sign', or 'product', got {other:?}"
            ))),
        },
    }
}

fn normalize_algorithm(raw: Option<&str>) -> AppResult<&'static str> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok("hnsw"),
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "hnsw" => Ok("hnsw"),
            "diskann" | "disk-ann" | "vamana" => Ok("diskann"),
            "ivf" => Ok("ivf"),
            other => Err(AppError::msg(format!(
                "algorithm must be 'hnsw', 'diskann', or 'ivf', got {other:?}"
            ))),
        },
    }
}

/// Engine-supported algorithm × quantization pairs (0.63+).
fn validate_ann_pair(algorithm: &str, quantization: &str) -> AppResult<()> {
    let ok = matches!(
        (algorithm, quantization),
        ("hnsw", "binary_sign")
            | ("hnsw", "dense")
            | ("hnsw", "product")
            | ("diskann", "dense")
            | ("ivf", "dense")
    );
    if ok {
        Ok(())
    } else {
        Err(AppError::msg(format!(
            "unsupported ANN pair algorithm={algorithm:?} quantization={quantization:?}; \
             supported: hnsw×{{binary_sign,dense,product}}, diskann×dense, ivf×dense"
        )))
    }
}

fn quantization_label(q: &str) -> &'static str {
    match q {
        "binary_sign" => "BinarySign",
        "product" => "Product",
        _ => "Dense",
    }
}

fn algorithm_label(a: &str) -> &'static str {
    match a {
        "diskann" => "DiskANN",
        "ivf" => "IVF",
        _ => "HNSW",
    }
}

fn existing_ann_quantization(db: &DbSession, table: &str, emb_col: &str) -> Option<&'static str> {
    let handle = db.database.table(table).ok()?;
    let guard = handle.lock();
    let schema = guard.schema();
    let emb_id = schema.columns.iter().find(|c| c.name == emb_col)?.id;
    let idx = schema
        .indexes
        .iter()
        .find(|idx| idx.kind == IndexKind::Ann && idx.column_id == emb_id)?;
    match idx.options.ann.as_ref().map(|o| &o.quantization) {
        Some(mongreldb_core::schema::AnnQuantization::Dense) => Some("dense"),
        Some(mongreldb_core::schema::AnnQuantization::Product { .. }) => Some("product"),
        Some(mongreldb_core::schema::AnnQuantization::BinarySign) | None => Some("binary_sign"),
    }
}

fn existing_ann_algorithm(db: &DbSession, table: &str, emb_col: &str) -> Option<&'static str> {
    use mongreldb_core::schema::AnnAlgorithm;
    let handle = db.database.table(table).ok()?;
    let guard = handle.lock();
    let schema = guard.schema();
    let emb_id = schema.columns.iter().find(|c| c.name == emb_col)?.id;
    let idx = schema
        .indexes
        .iter()
        .find(|idx| idx.kind == IndexKind::Ann && idx.column_id == emb_id)?;
    match idx.options.ann.as_ref().map(|o| o.algorithm) {
        Some(AnnAlgorithm::DiskAnn) => Some("diskann"),
        Some(AnnAlgorithm::Ivf) => Some("ivf"),
        Some(AnnAlgorithm::Hnsw) | None => Some("hnsw"),
    }
}

fn existing_ann_index_name(db: &DbSession, table: &str, emb_col: &str) -> Option<String> {
    let handle = db.database.table(table).ok()?;
    let guard = handle.lock();
    let schema = guard.schema();
    let emb_id = schema.columns.iter().find(|c| c.name == emb_col)?.id;
    schema
        .indexes
        .iter()
        .find(|idx| idx.kind == IndexKind::Ann && idx.column_id == emb_id)
        .map(|idx| idx.name.clone())
}

fn ensure_embedding_column_and_check_ann(
    db: &DbSession,
    table: &str,
    emb_col: &str,
    dim: u32,
) -> AppResult<bool> {
    let handle = db.database.table(table).map_err(AppError::db)?;
    let mut guard = handle.lock();
    let schema = guard.schema().clone();
    let existing = schema.columns.iter().find(|c| c.name == emb_col);
    match existing {
        Some(col) => match &col.ty {
            TypeId::Embedding { dim: d } if *d == dim => {}
            TypeId::Embedding { dim: d } => {
                return Err(AppError::msg(format!(
                    "column {emb_col} already exists as Embedding({d}); expected Embedding({dim})"
                )));
            }
            other => {
                return Err(AppError::msg(format!(
                    "column {emb_col} already exists with type {other:?}"
                )));
            }
        },
        None => {
            guard
                .add_column(
                    emb_col,
                    TypeId::Embedding { dim },
                    ColumnFlags::empty().with(ColumnFlags::NULLABLE),
                    None,
                )
                .map_err(AppError::db)?;
        }
    }

    let schema = guard.schema().clone();
    let emb_id = schema
        .columns
        .iter()
        .find(|c| c.name == emb_col)
        .map(|c| c.id);
    let has_ann = schema
        .indexes
        .iter()
        .any(|idx| idx.kind == IndexKind::Ann && emb_id.is_some_and(|id| idx.column_id == id));
    Ok(has_ann)
}

fn primary_key_name(db: &DbSession, table: &str) -> AppResult<String> {
    let handle = db.database.table(table).map_err(AppError::db)?;
    let guard = handle.lock();
    Ok(guard
        .schema()
        .columns
        .iter()
        .find(|c| c.flags.contains(ColumnFlags::PRIMARY_KEY))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "id".into()))
}

fn table_column_names(db: &DbSession, table: &str) -> AppResult<Vec<String>> {
    let handle = db.database.table(table).map_err(AppError::db)?;
    let guard = handle.lock();
    Ok(guard
        .schema()
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect())
}

fn require_column(db: &DbSession, table: &str, col: &str) -> AppResult<()> {
    let names = table_column_names(db, table)?;
    if names.iter().any(|n| n == col) {
        return Ok(());
    }
    Err(AppError::msg(format!(
        "Table `{table}` has no column `{col}`. Available columns: {}. \
         Pick a real text column for backfill (e.g. payload/kind on events, body on documents).",
        if names.is_empty() {
            "(none)".into()
        } else {
            names.join(", ")
        }
    )))
}

fn require_ann_surface(db: &DbSession, table: &str, emb_col: &str) -> AppResult<()> {
    require_column(db, table, emb_col)?;
    let handle = db.database.table(table).map_err(AppError::db)?;
    let guard = handle.lock();
    let schema = guard.schema();
    let emb_id = schema
        .columns
        .iter()
        .find(|c| c.name == emb_col)
        .map(|c| c.id);
    let has_ann = schema
        .indexes
        .iter()
        .any(|idx| idx.kind == IndexKind::Ann && emb_id.is_some_and(|id| idx.column_id == id));
    if has_ann {
        return Ok(());
    }
    Err(AppError::msg(format!(
        "Table `{table}` has no ANN index on `{emb_col}`. \
         Use Install ANN first (Dense f32 cosine by default; pick a text column that exists on this table)."
    )))
}

#[allow(clippy::too_many_arguments)]
async fn backfill_embeddings(
    db: &DbSession,
    embeddings: &EmbeddingHub,
    table: &str,
    text_col: &str,
    emb_col: &str,
    dim: u32,
    provider_id: Option<&str>,
    limit: usize,
) -> AppResult<u64> {
    require_column(db, table, text_col)?;
    require_column(db, table, emb_col)?;
    let pk_name = primary_key_name(db, table)?;
    let select = format!("SELECT {pk_name}, {text_col} FROM {table} LIMIT {limit}");
    let result = run_sql(
        db,
        SqlRequest {
            sql: select,
            max_rows: Some(limit),
        },
    )
    .await
    .map_err(|e| {
        AppError::sql(format!(
            "backfill select failed on `{table}.{text_col}`: {e}. \
             Choose a text column that exists on this table."
        ))
    })?;

    let mut updated = 0u64;
    for chunk in result.rows.chunks(32) {
        let mut texts = Vec::new();
        let mut keys = Vec::new();
        for row in chunk {
            if row.len() < 2 {
                continue;
            }
            let key = row[0].clone();
            let text = match &row[1] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => continue,
                other => other.to_string(),
            };
            if text.is_empty() {
                continue;
            }
            keys.push(key);
            texts.push(text);
        }
        if texts.is_empty() {
            continue;
        }
        let emb = embeddings.embed(&texts, provider_id)?;
        if emb.dimension != dim {
            return Err(AppError::Embedding(format!(
                "provider returned dim {}, expected {dim}",
                emb.dimension
            )));
        }

        for (key, vector) in keys.iter().zip(emb.vectors.iter()) {
            let vec_lit = format!(
                "[{}]",
                vector
                    .iter()
                    .map(|f| format!("{f}"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let key_sql = match key {
                serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                other => other.to_string(),
            };
            let update =
                format!("UPDATE {table} SET {emb_col} = '{vec_lit}' WHERE {pk_name} = {key_sql}");
            match db.session.run(&update).await {
                Ok(_) => updated += 1,
                Err(e) => tracing::warn!("backfill update failed for {key_sql}: {e}"),
            }
        }
    }

    Ok(updated)
}

/// Embed the query and build primary + fallback SQL.
///
/// When `exact_rerank` is true (default), primary uses `ann_search_exact` so
/// results are similarity-ranked - not table order from a raw HNSW prefilter.
/// HNSW prefilter width (`candidate_k`) is wider than the final limit so exact
/// cosine rerank can reorder meaningfully.
/// Fallback is always `WHERE ann_search(...)` if exact is unavailable.
pub fn plan_semantic_search(
    embeddings: &EmbeddingHub,
    req: &SemanticSearchRequest,
    exact_proj_cols: &str,
) -> AppResult<(usize, String, String)> {
    let k = req.k.unwrap_or(5).clamp(1, 1000);
    // Pull a wider HNSW candidate set, then exact-rerank down to k.
    let candidate_k = k.saturating_mul(20).clamp(k, 1000);
    let emb = embeddings.embed(std::slice::from_ref(&req.query), req.provider_id.as_deref())?;
    let vector = emb
        .vectors
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Embedding("empty embedding".into()))?;
    let vec_lit = format!(
        "[{}]",
        vector
            .iter()
            .map(|f| format!("{f}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let projection = req.projection.clone().unwrap_or_else(|| "*".into());

    let primary = if req.exact_rerank.unwrap_or(true) {
        format!(
            "SELECT * FROM ann_search_exact('{}', '{}', '{vec_lit}', {candidate_k}, {k}, 'cosine', '{exact_proj_cols}')",
            req.table, req.embedding_column
        )
    } else {
        format!(
            "SELECT {projection} FROM {} WHERE ann_search({}, '{vec_lit}', {k})",
            req.table, req.embedding_column
        )
    };
    let fallback = format!(
        "SELECT {projection} FROM {} WHERE ann_search({}, '{vec_lit}', {k})",
        req.table, req.embedding_column
    );
    Ok((k, primary, fallback))
}

/// Drop rows whose cosine similarity is below `min_score`.
///
/// Accepts `exact_score` (SQL ann_search_exact, higher better) or native
/// `score` when `score_kind` is `ann_cosine_distance` (lower better → convert
/// via `1 - distance`).
fn apply_min_score(mut result: SqlResult, min_score: Option<f32>) -> SqlResult {
    let Some(threshold) = min_score else {
        return result;
    };
    if threshold <= 0.0 {
        return result;
    }
    let exact_idx = result
        .columns
        .iter()
        .position(|c| c.eq_ignore_ascii_case("exact_score"));
    let score_idx = result
        .columns
        .iter()
        .position(|c| c.eq_ignore_ascii_case("score"));
    let kind_idx = result
        .columns
        .iter()
        .position(|c| c.eq_ignore_ascii_case("score_kind"));

    result.rows.retain(|row| {
        if let Some(idx) = exact_idx {
            return row
                .get(idx)
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                .map(|s| s as f32 >= threshold)
                .unwrap_or(true);
        }
        if let (Some(s_idx), Some(k_idx)) = (score_idx, kind_idx) {
            let kind = row.get(k_idx).and_then(|v| v.as_str()).unwrap_or("");
            if kind == "ann_cosine_distance" {
                let dist = row.get(s_idx).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                return (1.0 - dist) >= threshold;
            }
        }
        true
    });
    result.row_count = result.rows.len();
    result
}

/// Column list for `ann_search_exact` projection arg. Direct sessions inspect
/// the table schema; server falls back to a safe default.
pub fn resolve_exact_projection(db: Option<&DbSession>, req: &SemanticSearchRequest) -> String {
    let projection = req.projection.clone().unwrap_or_else(|| "*".into());
    if projection != "*" {
        return projection;
    }
    match db {
        Some(db) => guess_projection(db, &req.table),
        None => "id".into(),
    }
}

/// Direct-session path (used by install/tests and as the preferred branch of
/// the connection-aware runner).
///
/// Prefers MongrelDB 0.64 **native `retrieve_text`** (engine embeds under the
/// active semantic identity, returns provenance). Falls back to SQL
/// `ann_search_exact` when the native path is not ready (no provider registry
/// binding, older roots, etc.).
pub async fn semantic_search(
    db: &DbSession,
    embeddings: &EmbeddingHub,
    req: SemanticSearchRequest,
) -> AppResult<SqlResult> {
    require_ann_surface(db, &req.table, &req.embedding_column)?;

    match try_native_retrieve_text(db, embeddings, &req) {
        Ok(result) => return Ok(apply_min_score(result, req.min_score)),
        Err(e) => {
            tracing::debug!(
                "native retrieve_text unavailable on {}: {e}; falling back to SQL ann_search_exact",
                req.table
            );
        }
    }

    let proj_cols = resolve_exact_projection(Some(db), &req);
    let (k, sql, fallback) = plan_semantic_search(embeddings, &req, &proj_cols)?;

    let raw = match run_sql(
        db,
        SqlRequest {
            sql,
            max_rows: Some(k),
        },
    )
    .await
    {
        Ok(r) => r,
        Err(e1) => run_sql(
            db,
            SqlRequest {
                sql: fallback,
                max_rows: Some(k),
            },
        )
        .await
        .map_err(|e2| AppError::sql(format!("semantic search failed: {e1}; fallback: {e2}")))?,
    };
    let mut raw = apply_min_score(raw, req.min_score);
    if raw.search_mode.is_none() {
        raw.search_mode = Some("sql_ann_exact".into());
    }
    Ok(raw)
}

/// Engine-native text → embed under semantic identity → ANN (0.64+).
fn try_native_retrieve_text(
    db: &DbSession,
    embeddings: &EmbeddingHub,
    req: &SemanticSearchRequest,
) -> AppResult<SqlResult> {
    let started = std::time::Instant::now();
    let k = req.k.unwrap_or(5).clamp(1, 1000);

    // Ensure the process-local provider is loaded and registered on this root.
    if req.provider_id.as_deref().unwrap_or(DEFAULT_PROVIDER_ID) == DEFAULT_PROVIDER_ID {
        embeddings.ensure_local_default()?;
    }
    embeddings.register_on_database(&db.database)?;
    let _ = stamp_embedding_source(
        db,
        &req.table,
        &req.embedding_column,
        embeddings,
        req.provider_id.as_deref(),
    );

    let emb_col_id = embedding_column_id(db, &req.table, &req.embedding_column)?;
    let retrieved = db
        .database
        .retrieve_text(
            &req.table,
            emb_col_id,
            &req.query,
            TextSearchOptions::new(k),
        )
        .map_err(|e| AppError::msg(format!("retrieve_text: {e}")))?;

    let prov = &retrieved.provenance;
    let fp = prov.semantic_identity.fingerprint_sha256();
    let provenance = SearchProvenance {
        provider_id: prov.semantic_identity.provider_id.clone(),
        provider_version: prov.semantic_identity.provider_version.clone(),
        model_id: prov.semantic_identity.model_id.clone(),
        model_version: prov.semantic_identity.model_version.clone(),
        dimension: prov.semantic_identity.dimension,
        fingerprint_short: fp.iter().take(8).map(|b| format!("{b:02x}")).collect(),
        provider_registry_generation: prov.provider_registry_generation,
        embedding_column: req.embedding_column.clone(),
    };

    let (columns, rows) = hydrate_retrieve_hits(db, &req.table, &retrieved.hits)?;
    Ok(SqlResult {
        columns,
        rows,
        row_count: retrieved.hits.len(),
        truncated: false,
        elapsed_ms: started.elapsed().as_millis() as u64,
        statement_kind: "retrieve_text".into(),
        search_mode: Some("native_retrieve_text".into()),
        provenance: Some(provenance),
    })
}

fn stamp_embedding_source(
    db: &DbSession,
    table: &str,
    emb_col: &str,
    embeddings: &EmbeddingHub,
    provider_id: Option<&str>,
) -> AppResult<()> {
    let source = embeddings.configured_source(provider_id);
    // Skip if already set to a configured/local/generated source.
    {
        let handle = db.database.table(table).map_err(AppError::db)?;
        let guard = handle.lock();
        if let Some(col) = guard.schema().columns.iter().find(|c| c.name == emb_col) {
            match &col.embedding_source {
                Some(EmbeddingSource::ConfiguredModel { .. })
                | Some(EmbeddingSource::LocalModel { .. })
                | Some(EmbeddingSource::GeneratedColumn { .. })
                | Some(EmbeddingSource::GeneratedColumnSpec { .. }) => return Ok(()),
                _ => {}
            }
        }
    }
    db.database
        .alter_column(table, emb_col, AlterColumn::set_embedding_source(source))
        .map_err(AppError::db)?;
    Ok(())
}

fn embedding_column_id(db: &DbSession, table: &str, emb_col: &str) -> AppResult<u16> {
    let handle = db.database.table(table).map_err(AppError::db)?;
    let guard = handle.lock();
    guard
        .schema()
        .columns
        .iter()
        .find(|c| c.name == emb_col)
        .map(|c| c.id)
        .ok_or_else(|| {
            AppError::msg(format!(
                "embedding column `{emb_col}` not found on `{table}`"
            ))
        })
}

fn hydrate_retrieve_hits(
    db: &DbSession,
    table: &str,
    hits: &[mongreldb_core::query::RetrieverHit],
) -> AppResult<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
    let handle = db.database.table(table).map_err(AppError::db)?;
    let guard = handle.lock();
    let schema = guard.schema().clone();
    let snapshot = guard.snapshot();

    let col_meta: Vec<(u16, String)> = schema
        .columns
        .iter()
        .filter(|c| !matches!(c.ty, TypeId::Embedding { .. }))
        .take(10)
        .map(|c| (c.id, c.name.clone()))
        .collect();
    // Score metadata first so ranking is obvious in the Hits table.
    let mut columns = vec![
        "rank".into(),
        "score_kind".into(),
        "score".into(),
        "row_id".into(),
    ];
    for (_, name) in &col_meta {
        columns.push(name.clone());
    }

    let mut rows = Vec::with_capacity(hits.len());
    for hit in hits {
        let (score_kind, score_value) = match hit.score {
            mongreldb_core::query::RetrieverScore::AnnHammingDistance(d) => {
                ("ann_hamming_distance", f64::from(d))
            }
            mongreldb_core::query::RetrieverScore::AnnCosineDistance(d) => {
                ("ann_cosine_distance", f64::from(d))
            }
            mongreldb_core::query::RetrieverScore::SparseDotProduct(v) => ("sparse_dot_product", v),
            mongreldb_core::query::RetrieverScore::MinHashEstimatedJaccard(v) => {
                ("minhash_estimated_jaccard", f64::from(v))
            }
        };
        let mut row = vec![
            serde_json::json!(hit.rank),
            serde_json::json!(score_kind),
            serde_json::json!(score_value),
            serde_json::json!(hit.row_id.0),
        ];
        if let Some(core_row) = guard.get(hit.row_id, snapshot) {
            for (id, _) in &col_meta {
                let cell = core_row
                    .columns
                    .get(id)
                    .map(core_value_json)
                    .unwrap_or(serde_json::Value::Null);
                row.push(cell);
            }
        } else {
            for _ in &col_meta {
                row.push(serde_json::Value::Null);
            }
        }
        rows.push(row);
    }
    Ok((columns, rows))
}

fn core_value_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::json!(b),
        Value::Int64(n) => serde_json::json!(n),
        Value::Float64(f) => serde_json::json!(f),
        Value::Bytes(b) => match std::str::from_utf8(b) {
            Ok(s) => serde_json::json!(s),
            Err(_) => serde_json::json!(format!(
                "\\x{}",
                b.iter().map(|x| format!("{x:02x}")).collect::<String>()
            )),
        },
        Value::Embedding(e) => serde_json::json!(format!("[{}d embedding]", e.len())),
        Value::GeneratedEmbedding(e) => {
            serde_json::json!(format!("[{}d generated embedding]", e.vector.len()))
        }
        Value::Decimal(d) => serde_json::json!(d.to_string()),
        Value::Interval {
            months,
            days,
            nanos,
        } => {
            serde_json::json!(format!("interval({months}m {days}d {nanos}ns)"))
        }
        Value::Uuid(u) => {
            let s = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                u[0], u[1], u[2], u[3], u[4], u[5], u[6], u[7],
                u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
            );
            serde_json::json!(s)
        }
        Value::Json(b) => match std::str::from_utf8(b) {
            Ok(s) => serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!(s)),
            Err(_) => serde_json::Value::Null,
        },
    }
}

/// Shared path for Tauri commands and MCP: Direct → full `semantic_search`;
/// Server → same SQL plan via HTTP `sql_work`.
pub async fn semantic_search_on_connection(
    db: &crate::db::connection::SharedConnection,
    embeddings: &EmbeddingHub,
    req: SemanticSearchRequest,
) -> AppResult<crate::models::SqlResult> {
    // Prefer direct: clone session handles under the lock, then run unlocked.
    let direct = {
        let guard = db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        match conn {
            crate::db::connection::Connection::Direct(d) => Some(DbSession {
                path: d.path.clone(),
                database: std::sync::Arc::clone(&d.database),
                session: std::sync::Arc::clone(&d.session),
                opened_at: d.opened_at,
                credentials_required: d.credentials_required,
            }),
            crate::db::connection::Connection::Server(_) => None,
        }
    };

    if let Some(direct) = direct {
        return semantic_search(&direct, embeddings, req).await;
    }

    // Server: same SQL semantics (exact_rerank → ann_search_exact).
    let proj_cols = resolve_exact_projection(None, &req);
    let (k, primary_sql, fallback_sql) = plan_semantic_search(embeddings, &req, &proj_cols)?;
    let (primary, fallback) = {
        let guard = db.read();
        let conn = guard.as_ref().ok_or(AppError::NoDatabase)?;
        (
            conn.sql_work(SqlRequest {
                sql: primary_sql,
                max_rows: Some(k),
            })?,
            conn.sql_work(SqlRequest {
                sql: fallback_sql,
                max_rows: Some(k),
            })?,
        )
    };
    let raw = match primary.run().await {
        Ok(r) => r,
        Err(e1) => fallback
            .run()
            .await
            .map_err(|e2| AppError::sql(format!("semantic search failed: {e1}; fallback: {e2}")))?,
    };
    let mut raw = apply_min_score(raw, req.min_score);
    raw.search_mode = Some("sql_ann_exact".into());
    Ok(raw)
}

fn guess_projection(db: &DbSession, table: &str) -> String {
    let Ok(handle) = db.database.table(table) else {
        return "id".into();
    };
    let guard = handle.lock();
    let names: Vec<String> = guard
        .schema()
        .columns
        .iter()
        .filter(|c| !matches!(c.ty, TypeId::Embedding { .. }))
        .take(8)
        .map(|c| c.name.clone())
        .collect();
    if names.is_empty() {
        "id".into()
    } else {
        names.join(",")
    }
}
