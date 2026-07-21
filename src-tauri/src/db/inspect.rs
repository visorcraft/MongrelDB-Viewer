use mongreldb_core::schema::{
    AnnQuantization, IndexKind, IndexOptions, TypeId,
};
use mongreldb_core::EmbeddingSource;

use crate::db::session::DbSession;
use crate::error::{AppError, AppResult};
use crate::models::{
    AnnIndexOptions, ColumnInfo, ConstellationGraph, DatabaseOverview, ForeignKeyInfo, GraphEdge,
    GraphNode, IndexInfo, IndexRadar, TableDetail, TableSummary,
};

pub fn database_overview(db: &DbSession) -> AppResult<DatabaseOverview> {
    let engine = mongreldb_core::build_info();
    let query = mongreldb_query::build_info();
    let names = db.database.table_names();
    let mut tables = Vec::with_capacity(names.len());
    for name in &names {
        tables.push(table_summary(db, name)?);
    }
    tables.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(DatabaseOverview {
        path: db.path.display().to_string(),
        connection_mode: "direct".into(),
        opened_at: db.opened_at.to_rfc3339(),
        engine_version: engine.engine_version.to_string(),
        query_version: query.query_version.to_string(),
        git_sha: engine.mongreldb_git_sha.to_string(),
        table_count: tables.len(),
        tables,
        embedding_providers: Vec::new(), // filled by commands layer
        credentials_required: db.credentials_required,
        display_label: db.path.display().to_string(),
    })
}

pub fn table_summary(db: &DbSession, name: &str) -> AppResult<TableSummary> {
    let handle = db.database.table(name).map_err(AppError::db)?;
    let guard = handle.lock();
    let schema = guard.schema().clone();
    let row_count = guard.count();
    drop(guard);

    let mut has_ann = false;
    let mut has_sparse = false;
    let mut has_minhash = false;
    let mut has_fm = false;
    let mut has_bitmap = false;
    let mut has_learned_range = false;
    for idx in &schema.indexes {
        match idx.kind {
            IndexKind::Ann => has_ann = true,
            IndexKind::Sparse => has_sparse = true,
            IndexKind::MinHash => has_minhash = true,
            IndexKind::FmIndex => has_fm = true,
            IndexKind::Bitmap => has_bitmap = true,
            IndexKind::LearnedRange => has_learned_range = true,
        }
    }
    let embedding_dims: Vec<u32> = schema
        .columns
        .iter()
        .filter_map(|c| match c.ty {
            TypeId::Embedding { dim } => Some(dim),
            _ => None,
        })
        .collect();

    Ok(TableSummary {
        name: name.to_string(),
        row_count,
        column_count: schema.columns.len(),
        index_count: schema.indexes.len(),
        has_ann,
        has_sparse,
        has_minhash,
        has_fm,
        has_bitmap,
        has_learned_range,
        embedding_dims,
    })
}

pub fn table_detail(db: &DbSession, name: &str) -> AppResult<TableDetail> {
    let handle = db.database.table(name).map_err(AppError::db)?;
    let guard = handle.lock();
    let schema = guard.schema().clone();
    let row_count = guard.count();
    drop(guard);

    let columns: Vec<ColumnInfo> = schema
        .columns
        .iter()
        .map(|c| {
            let mut flags = Vec::new();
            if c.flags.contains(ColumnFlagsBits::PRIMARY_KEY) {
                flags.push("PRIMARY_KEY".into());
            }
            if c.flags.contains(ColumnFlagsBits::NULLABLE) {
                flags.push("NULLABLE".into());
            }
            if c.flags.contains(ColumnFlagsBits::ENCRYPTED) {
                flags.push("ENCRYPTED".into());
            }
            if c.flags.contains(ColumnFlagsBits::AUTO_INCREMENT) {
                flags.push("AUTO_INCREMENT".into());
            }
            if c.flags.contains(ColumnFlagsBits::EMBEDDING_BINARY_QUANTIZED) {
                flags.push("EMBEDDING_BINARY_QUANTIZED".into());
            }
            ColumnInfo {
                id: c.id,
                name: c.name.clone(),
                type_name: type_name(&c.ty),
                flags,
                embedding_dim: match c.ty {
                    TypeId::Embedding { dim } => Some(dim),
                    _ => None,
                },
                embedding_source: c
                    .embedding_source
                    .as_ref()
                    .map(describe_embedding_source),
            }
        })
        .collect();

    let col_name = |id: u16| {
        schema
            .columns
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("col_{id}"))
    };

    let indexes: Vec<IndexInfo> = schema
        .indexes
        .iter()
        .map(|idx| {
            index_info_from_parts(
                idx.name.clone(),
                idx.column_id,
                col_name(idx.column_id),
                index_kind_name(idx.kind).into(),
                idx.predicate.clone(),
                &idx.options,
            )
        })
        .collect();

    let mut radar = IndexRadar {
        bitmap: 0,
        learned_range: 0,
        fm_index: 0,
        ann: 0,
        sparse: 0,
        minhash: 0,
    };
    for idx in &schema.indexes {
        match idx.kind {
            IndexKind::Bitmap => radar.bitmap += 1,
            IndexKind::LearnedRange => radar.learned_range += 1,
            IndexKind::FmIndex => radar.fm_index += 1,
            IndexKind::Ann => radar.ann += 1,
            IndexKind::Sparse => radar.sparse += 1,
            IndexKind::MinHash => radar.minhash += 1,
        }
    }

    let foreign_keys: Vec<ForeignKeyInfo> = schema
        .constraints
        .foreign_keys
        .iter()
        .map(|fk| ForeignKeyInfo {
            name: fk.name.clone(),
            columns: fk.columns.iter().map(|id| col_name(*id)).collect(),
            ref_table: fk.ref_table.clone(),
            ref_columns: fk
                .ref_columns
                .iter()
                .map(|id| format!("col_{id}"))
                .collect(),
            // Prefer resolving ref column names when parent is known - filled below if possible.
        })
        .collect();

    // Resolve referenced column names from parent table schemas when available.
    let mut foreign_keys = foreign_keys;
    for fk in &mut foreign_keys {
        if let Ok(parent) = db.database.table(&fk.ref_table) {
            let parent_schema = parent.lock().schema().clone();
            if let Some(src) = schema
                .constraints
                .foreign_keys
                .iter()
                .find(|f| f.name == fk.name)
            {
                fk.ref_columns = src
                    .ref_columns
                    .iter()
                    .map(|id| {
                        parent_schema
                            .columns
                            .iter()
                            .find(|c| c.id == *id)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| format!("col_{id}"))
                    })
                    .collect();
            }
        }
    }

    Ok(TableDetail {
        name: name.to_string(),
        schema_id: schema.schema_id,
        row_count,
        columns,
        indexes,
        index_radar: radar,
        foreign_keys,
    })
}

pub fn build_constellation(db: &DbSession) -> AppResult<ConstellationGraph> {
    let names = db.database.table_names();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(GraphNode {
        id: "db".into(),
        label: db
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("database")
            .to_string(),
        kind: "database".into(),
        meta: serde_json::json!({
            "path": db.path.display().to_string(),
            "tables": names.len(),
        }),
    });

    for name in names {
        let detail = table_detail(db, &name)?;
        let table_id = format!("table:{name}");
        nodes.push(GraphNode {
            id: table_id.clone(),
            label: name.clone(),
            kind: "table".into(),
            meta: serde_json::json!({
                "rows": detail.row_count,
                "columns": detail.columns.len(),
                "indexes": detail.indexes.len(),
                "hasAnn": detail.index_radar.ann > 0,
            }),
        });
        edges.push(GraphEdge {
            from: "db".into(),
            to: table_id.clone(),
            kind: "owns".into(),
        });

        for col in &detail.columns {
            let col_id = format!("col:{name}:{}", col.name);
            nodes.push(GraphNode {
                id: col_id.clone(),
                label: col.name.clone(),
                kind: if col.embedding_dim.is_some() {
                    "embedding".into()
                } else {
                    "column".into()
                },
                meta: serde_json::json!({
                    "type": col.type_name,
                    "flags": col.flags,
                    "dim": col.embedding_dim,
                    "embeddingSource": col.embedding_source,
                }),
            });
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: col_id,
                kind: "column".into(),
            });
        }

        for idx in &detail.indexes {
            let idx_id = format!("idx:{name}:{}", idx.name);
            let label = match &idx.options_summary {
                Some(s) => format!("{} ({}) · {}", idx.name, idx.kind, s),
                None => format!("{} ({})", idx.name, idx.kind),
            };
            nodes.push(GraphNode {
                id: idx_id.clone(),
                label,
                kind: "index".into(),
                meta: serde_json::json!({
                    "kind": idx.kind,
                    "column": idx.column_name,
                    "options": idx.options_summary,
                    "ann": idx.ann,
                }),
            });
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: idx_id.clone(),
                kind: "index".into(),
            });
            edges.push(GraphEdge {
                from: idx_id,
                to: format!("col:{name}:{}", idx.column_name),
                kind: "covers".into(),
            });
        }

        // Table-to-table foreign key edges for the schema map.
        for fk in &detail.foreign_keys {
            edges.push(GraphEdge {
                from: table_id.clone(),
                to: format!("table:{}", fk.ref_table),
                kind: "fk".into(),
            });
            // Also link the local FK column node to the parent table.
            if let Some(local_col) = fk.columns.first() {
                edges.push(GraphEdge {
                    from: format!("col:{name}:{local_col}"),
                    to: format!("table:{}", fk.ref_table),
                    kind: "fk-col".into(),
                });
            }
        }
    }

    Ok(ConstellationGraph { nodes, edges })
}

fn type_name(ty: &TypeId) -> String {
    match ty {
        TypeId::Bool => "Bool".into(),
        TypeId::Int8 => "Int8".into(),
        TypeId::Int16 => "Int16".into(),
        TypeId::Int32 => "Int32".into(),
        TypeId::Int64 => "Int64".into(),
        TypeId::UInt8 => "UInt8".into(),
        TypeId::UInt16 => "UInt16".into(),
        TypeId::UInt32 => "UInt32".into(),
        TypeId::UInt64 => "UInt64".into(),
        TypeId::Float32 => "Float32".into(),
        TypeId::Float64 => "Float64".into(),
        TypeId::TimestampNanos => "TimestampNanos".into(),
        TypeId::Date32 => "Date32".into(),
        TypeId::Date64 => "Date64".into(),
        TypeId::Time64 => "Time64".into(),
        TypeId::Interval => "Interval".into(),
        TypeId::Uuid => "Uuid".into(),
        TypeId::Json => "Json".into(),
        TypeId::Array { element_type } => format!("Array({element_type})"),
        TypeId::Bytes => "Bytes".into(),
        TypeId::Embedding { dim } => format!("Embedding({dim})"),
        TypeId::Decimal128 { precision, scale } => format!("Decimal({precision},{scale})"),
        TypeId::Enum { variants } => format!("Enum({} variants)", variants.len()),
    }
}

fn index_kind_name(kind: IndexKind) -> &'static str {
    match kind {
        IndexKind::Bitmap => "Bitmap",
        IndexKind::FmIndex => "FmIndex",
        IndexKind::Ann => "Ann",
        IndexKind::LearnedRange => "LearnedRange",
        IndexKind::MinHash => "MinHash",
        IndexKind::Sparse => "Sparse",
    }
}

/// Build [`IndexInfo`] from shared direct/server field pieces.
pub fn index_info_from_parts(
    name: String,
    column_id: u16,
    column_name: String,
    kind: String,
    predicate: Option<String>,
    options: &IndexOptions,
) -> IndexInfo {
    let is_ann = kind.to_ascii_lowercase().contains("ann");
    let ann = if is_ann {
        Some(ann_options_info(options))
    } else {
        None
    };
    let options_summary = options_summary_for(&kind, options);
    IndexInfo {
        name,
        column_id,
        column_name,
        kind,
        predicate,
        ann,
        options_summary,
    }
}

/// ANN options from schema, or engine defaults when omitted (BinarySign).
fn ann_options_info(options: &IndexOptions) -> AnnIndexOptions {
    match options.ann.as_ref() {
        Some(ann) => AnnIndexOptions {
            m: ann.m,
            ef_construction: ann.ef_construction,
            ef_search: ann.ef_search,
            quantization: quantization_name(ann.quantization).into(),
        },
        None => AnnIndexOptions {
            m: 16,
            ef_construction: 64,
            ef_search: 64,
            quantization: "binary_sign".into(),
        },
    }
}

fn quantization_name(q: AnnQuantization) -> &'static str {
    match q {
        AnnQuantization::Dense => "dense",
        AnnQuantization::BinarySign => "binary_sign",
    }
}

fn options_summary_for(kind: &str, options: &IndexOptions) -> Option<String> {
    let k = kind.to_ascii_lowercase();
    if k.contains("ann") {
        if let Some(ann) = &options.ann {
            return Some(format!(
                "{} · m={} · efc={} · efs={}",
                quantization_name(ann.quantization),
                ann.m,
                ann.ef_construction,
                ann.ef_search
            ));
        }
        // Schema omitted options → engine default is BinarySign.
        return Some("binary_sign (default)".into());
    }
    if k.contains("minhash") {
        if let Some(mh) = &options.minhash {
            return Some(format!(
                "permutations={} · bands={}",
                mh.permutations, mh.bands
            ));
        }
    }
    if k.contains("learned") || k.contains("range") || k.contains("pgm") {
        if let Some(lr) = &options.learned_range {
            return Some(format!("epsilon={}", lr.epsilon));
        }
    }
    None
}

/// Rich description of an embedding column's vector source.
pub fn describe_embedding_source(src: &EmbeddingSource) -> String {
    match src {
        EmbeddingSource::SuppliedByApplication => "supplied_by_application".into(),
        EmbeddingSource::ConfiguredModel {
            provider_id,
            model_id,
            model_version,
        } => format!("configured_model · {provider_id} / {model_id} @ {model_version}"),
        EmbeddingSource::LocalModel { model_id, .. } => {
            format!("local_model · {model_id}")
        }
        EmbeddingSource::GeneratedColumn { provider } => {
            format!("generated_column · {provider}")
        }
        EmbeddingSource::GeneratedColumnSpec { spec } => format!(
            "generated_column · {} / {} @ {}",
            spec.provider_id, spec.model_id, spec.model_version
        ),
    }
}

/// Local helper so we don't need to re-export ColumnFlags constants awkwardly.
struct ColumnFlagsBits;
impl ColumnFlagsBits {
    const NULLABLE: u32 = mongreldb_core::schema::ColumnFlags::NULLABLE;
    const PRIMARY_KEY: u32 = mongreldb_core::schema::ColumnFlags::PRIMARY_KEY;
    const ENCRYPTED: u32 = mongreldb_core::schema::ColumnFlags::ENCRYPTED;
    const AUTO_INCREMENT: u32 = mongreldb_core::schema::ColumnFlags::AUTO_INCREMENT;
    const EMBEDDING_BINARY_QUANTIZED: u32 =
        mongreldb_core::schema::ColumnFlags::EMBEDDING_BINARY_QUANTIZED;
}
