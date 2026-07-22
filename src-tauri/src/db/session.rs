use std::path::{Path, PathBuf};
use std::sync::Arc;

use mongreldb_core::constraint::{ForeignKey, TableConstraints};
use mongreldb_core::schema::{
    AnnAlgorithm as CoreAnnAlgorithm, AnnOptions, AnnQuantization as CoreAnnQuantization,
    ColumnDef, ColumnFlags, IndexDef, IndexKind as CoreIndexKind, IndexOptions, Schema, TypeId,
};
use mongreldb_core::{Database, EmbeddingSource, Value};
use mongreldb_kit::{
    AnnAlgorithm as KitAnnAlgorithm, AnnQuantization as KitAnnQuantization, Column as KitColumn,
    ColumnType as KitColumnType, ForeignKey as KitForeignKey, ForeignKeyAction as KitFkAction,
    Index as KitIndex, IndexKind as KitIndexKind, Schema as KitSchema, Table as KitTable,
};
use mongreldb_query::MongrelSession;

use crate::error::{AppError, AppResult};

/// Sidecar file written next to the engine root so Kit-backed clients (Mongrel
/// desktop, Kit HTTP, etc.) can open databases created by this Viewer.
const KIT_SCHEMA_FILE: &str = "kit_schema.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Open,
    Create,
}

/// Exclusive MongrelDB handle owned by the viewer process.
pub struct DbSession {
    pub path: PathBuf,
    pub database: Arc<Database>,
    pub session: Arc<MongrelSession>,
    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub credentials_required: bool,
}

impl DbSession {
    pub fn open(
        path: impl AsRef<Path>,
        username: Option<&str>,
        password: Option<&str>,
        passphrase: Option<&str>,
        mode: OpenMode,
    ) -> AppResult<Self> {
        let path = path.as_ref().to_path_buf();
        if mode == OpenMode::Create {
            if path.exists() {
                let is_empty = std::fs::read_dir(&path)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false);
                if !is_empty && path.is_dir() {
                    // Allow re-open of existing root as create_if_missing path.
                } else if path.is_file() {
                    return Err(AppError::msg(format!(
                        "path exists and is not an empty directory: {}",
                        path.display()
                    )));
                }
            } else {
                std::fs::create_dir_all(&path)?;
            }
        }

        let credentials_required = username.is_some();
        let database = match (mode, passphrase, username, password) {
            (OpenMode::Create, Some(pass), Some(user), Some(pw)) => {
                Database::create_encrypted_with_credentials(&path, pass, user, pw).map_err(AppError::db)?
            }
            (OpenMode::Create, Some(pass), None, None) => {
                Database::create_encrypted(&path, pass).map_err(AppError::db)?
            }
            (OpenMode::Create, None, Some(user), Some(pw)) => {
                Database::create_with_credentials(&path, user, pw).map_err(AppError::db)?
            }
            (OpenMode::Create, None, None, None) => {
                if path.join("_meta").exists() || path.join("tables").exists() {
                    // Root already looks like a MongrelDB - open instead.
                    open_existing(&path, username, password, passphrase)?
                } else {
                    Database::create(&path).map_err(AppError::db)?
                }
            }
            (OpenMode::Open, Some(pass), Some(user), Some(pw)) => {
                Database::open_encrypted_with_credentials(&path, pass, user, pw).map_err(AppError::db)?
            }
            (OpenMode::Open, Some(pass), None, None) => {
                Database::open_encrypted(&path, pass).map_err(AppError::db)?
            }
            (OpenMode::Open, None, Some(user), Some(pw)) => {
                Database::open_with_credentials(&path, user, pw).map_err(AppError::db)?
            }
            (OpenMode::Open, None, None, None) => Database::open(&path).map_err(AppError::db)?,
            _ => {
                return Err(AppError::msg(
                    "invalid credential combination (provide username+password together, and passphrase alone or with credentials)",
                ));
            }
        };

        let database = Arc::new(database);
        let session = Arc::new(MongrelSession::open(Arc::clone(&database)).map_err(AppError::sql)?);

        Ok(Self {
            path,
            database,
            session,
            opened_at: chrono::Utc::now(),
            credentials_required,
        })
    }

    pub fn create_demo(path: impl AsRef<Path>, with_ann: bool) -> AppResult<Self> {
        let path = path.as_ref();
        if looks_like_mongreldb(path) {
            return Err(AppError::msg(format!(
                "Refusing to create a demo database: {} already looks like a MongrelDB root (CATALOG / _meta / tables). Open it instead - demo create will not overwrite existing data.",
                path.display()
            )));
        }
        if path.exists() {
            let non_empty = std::fs::read_dir(path)
                .map(|mut d| d.next().is_some())
                .unwrap_or(true);
            if non_empty {
                return Err(AppError::msg(format!(
                    "Demo path must be empty or missing (won’t touch existing files): {}",
                    path.display()
                )));
            }
        } else {
            std::fs::create_dir_all(path)?;
        }

        let db = Database::create(path).map_err(AppError::db)?;
        seed_demo_schema(&db, with_ann).map_err(AppError::db)?;
        // Core create alone leaves no kit_schema.json. Kit-backed clients
        // (notably Mongrel) refuse to open a root without that sidecar.
        write_kit_schema_sidecar(path, &db)?;
        drop(db);
        Self::open(path, None, None, None, OpenMode::Open)
    }
}

fn col(id: u16, name: &str, ty: TypeId, flags: ColumnFlags) -> ColumnDef {
    col_with_source(id, name, ty, flags, None)
}

fn col_with_source(
    id: u16,
    name: &str,
    ty: TypeId,
    flags: ColumnFlags,
    embedding_source: Option<EmbeddingSource>,
) -> ColumnDef {
    ColumnDef {
        id,
        name: name.into(),
        ty,
        flags,
        default_value: None,
        embedding_source,
    }
}

fn pk(id: u16, name: &str) -> ColumnDef {
    col(
        id,
        name,
        TypeId::Int64,
        ColumnFlags::empty().with(ColumnFlags::PRIMARY_KEY),
    )
}

fn idx(name: &str, column_id: u16, kind: CoreIndexKind) -> IndexDef {
    IndexDef {
        name: name.into(),
        column_id,
        kind,
        predicate: None,
        options: IndexOptions::default(),
    }
}

/// Dense f32 cosine HNSW ANN (0.62+; algorithm explicit since 0.63).
/// Does not use BinarySign engine defaults.
fn dense_ann_idx(name: &str, column_id: u16) -> IndexDef {
    IndexDef {
        name: name.into(),
        column_id,
        kind: CoreIndexKind::Ann,
        predicate: None,
        options: IndexOptions {
            ann: Some(AnnOptions {
                m: 16,
                ef_construction: 64,
                ef_search: 64,
                quantization: CoreAnnQuantization::Dense,
                ..Default::default()
            }),
            ..IndexOptions::default()
        },
    }
}

fn fk(id: u16, name: &str, columns: Vec<u16>, ref_table: &str, ref_columns: Vec<u16>) -> ForeignKey {
    ForeignKey {
        id,
        name: name.into(),
        columns,
        ref_table: ref_table.into(),
        ref_columns,
        on_delete: Default::default(),
        on_update: Default::default(),
    }
}

fn bytes(s: &str) -> Value {
    Value::Bytes(s.as_bytes().to_vec())
}

/// Multi-table demo graph with real foreign keys so the schema map has edges
/// between tables - not just “owns” spokes from the database node.
fn seed_demo_schema(db: &Database, with_ann: bool) -> Result<(), mongreldb_core::MongrelError> {
    // tenants ──< authors
    //    │           │
    //    └─────< documents ──< events
    //                │
    //                └──── < document_tags > ── tags

    db.create_table(
        "tenants",
        Schema {
            columns: vec![
                pk(1, "id"),
                col(2, "name", TypeId::Bytes, ColumnFlags::empty()),
                col(3, "plan", TypeId::Bytes, ColumnFlags::empty()),
            ],
            indexes: vec![idx("tenants_plan_bm", 3, CoreIndexKind::Bitmap)],
            ..Schema::default()
        },
    )?;

    db.create_table(
        "authors",
        Schema {
            columns: vec![
                pk(1, "id"),
                col(2, "tenant_id", TypeId::Int64, ColumnFlags::empty()),
                col(3, "name", TypeId::Bytes, ColumnFlags::empty()),
                col(4, "role", TypeId::Bytes, ColumnFlags::empty()),
            ],
            indexes: vec![
                idx("authors_tenant_bm", 2, CoreIndexKind::Bitmap),
                idx("authors_role_bm", 4, CoreIndexKind::Bitmap),
            ],
            constraints: TableConstraints {
                foreign_keys: vec![fk(1, "authors_tenant_fk", vec![2], "tenants", vec![1])],
                ..Default::default()
            },
            ..Schema::default()
        },
    )?;

    let mut doc_cols = vec![
        pk(1, "id"),
        col(2, "tenant_id", TypeId::Int64, ColumnFlags::empty()),
        col(3, "author_id", TypeId::Int64, ColumnFlags::empty()),
        col(4, "body", TypeId::Bytes, ColumnFlags::empty()),
        col(5, "status", TypeId::Bytes, ColumnFlags::empty()),
        col(6, "score", TypeId::Float64, ColumnFlags::empty()),
    ];
    let mut doc_indexes = vec![
        idx("docs_tenant_bm", 2, CoreIndexKind::Bitmap),
        idx("docs_author_bm", 3, CoreIndexKind::Bitmap),
        idx("docs_body_fm", 4, CoreIndexKind::FmIndex),
        idx("docs_status_bm", 5, CoreIndexKind::Bitmap),
        idx("docs_score_pgm", 6, CoreIndexKind::LearnedRange),
    ];
    if with_ann {
        // 0.64+: stamp configured_model so native retrieve_text can resolve
        // the Viewer's MiniLM provider after register_on_database.
        doc_cols.push(col_with_source(
            7,
            "embedding",
            TypeId::Embedding { dim: 384 },
            ColumnFlags::empty().with(ColumnFlags::NULLABLE),
            Some(EmbeddingSource::ConfiguredModel {
                provider_id: crate::embeddings::DEFAULT_PROVIDER_ID.into(),
                model_id: crate::embeddings::DEFAULT_MODEL_ID.into(),
                model_version: crate::embeddings::DEFAULT_MODEL_VERSION.into(),
            }),
        ));
        doc_indexes.push(dense_ann_idx("docs_ann", 7));
    }
    db.create_table(
        "documents",
        Schema {
            columns: doc_cols,
            indexes: doc_indexes,
            constraints: TableConstraints {
                foreign_keys: vec![
                    fk(1, "docs_tenant_fk", vec![2], "tenants", vec![1]),
                    fk(2, "docs_author_fk", vec![3], "authors", vec![1]),
                ],
                ..Default::default()
            },
            ..Schema::default()
        },
    )?;

    db.create_table(
        "events",
        Schema {
            columns: vec![
                pk(1, "id"),
                col(2, "document_id", TypeId::Int64, ColumnFlags::empty()),
                col(3, "tenant_id", TypeId::Int64, ColumnFlags::empty()),
                col(4, "kind", TypeId::Bytes, ColumnFlags::empty()),
                col(5, "payload", TypeId::Json, ColumnFlags::empty()),
                col(6, "ts", TypeId::Int64, ColumnFlags::empty()),
            ],
            indexes: vec![
                idx("events_document_bm", 2, CoreIndexKind::Bitmap),
                idx("events_tenant_bm", 3, CoreIndexKind::Bitmap),
                idx("events_kind_bm", 4, CoreIndexKind::Bitmap),
                idx("events_ts_pgm", 6, CoreIndexKind::LearnedRange),
            ],
            constraints: TableConstraints {
                foreign_keys: vec![
                    fk(1, "events_document_fk", vec![2], "documents", vec![1]),
                    fk(2, "events_tenant_fk", vec![3], "tenants", vec![1]),
                ],
                ..Default::default()
            },
            ..Schema::default()
        },
    )?;

    db.create_table(
        "tags",
        Schema {
            columns: vec![
                pk(1, "id"),
                col(2, "name", TypeId::Bytes, ColumnFlags::empty()),
            ],
            indexes: vec![idx("tags_name_bm", 2, CoreIndexKind::Bitmap)],
            ..Schema::default()
        },
    )?;

    db.create_table(
        "document_tags",
        Schema {
            columns: vec![
                pk(1, "id"),
                col(2, "document_id", TypeId::Int64, ColumnFlags::empty()),
                col(3, "tag_id", TypeId::Int64, ColumnFlags::empty()),
            ],
            indexes: vec![
                idx("dt_document_bm", 2, CoreIndexKind::Bitmap),
                idx("dt_tag_bm", 3, CoreIndexKind::Bitmap),
            ],
            constraints: TableConstraints {
                foreign_keys: vec![
                    fk(1, "dt_document_fk", vec![2], "documents", vec![1]),
                    fk(2, "dt_tag_fk", vec![3], "tags", vec![1]),
                ],
                ..Default::default()
            },
            ..Schema::default()
        },
    )?;

    let samples = [
        (
            1i64,
            1i64,
            1i64,
            "MongrelDB stores operational rows and model-derived signals in one transactional row.",
            "active",
            0.97f64,
        ),
        (
            2,
            1,
            1,
            "Six public secondary indexes share one RowId space: Bitmap, PGM, FM, ANN, Sparse, MinHash.",
            "active",
            0.93,
        ),
        (
            3,
            1,
            2,
            "ANN algorithms: HNSW, DiskANN, IVF; quantizations: Dense, BinarySign, Product (0.63+).",
            "draft",
            0.88,
        ),
        (
            4,
            2,
            3,
            "SQL rides DataFusion 54 with recursive CTEs, windows, and scored AI table functions.",
            "active",
            0.91,
        ),
        (
            5,
            2,
            3,
            "The write path is WAL → Bε-tree memtable → immutable .sr columnar sorted runs.",
            "archived",
            0.84,
        ),
        (
            6,
            2,
            4,
            "Hybrid retrieval fuses dense, sparse, lexical, and metadata filters with RRF.",
            "active",
            0.95,
        ),
        (
            7,
            1,
            2,
            "Agent memory can be filtered by meaning, entities, recency, tenant, and type.",
            "active",
            0.9,
        ),
        (
            8,
            2,
            4,
            "MongrelDB Viewer is a Signal Deck for seeing every index, vector, and query path.",
            "active",
            0.99,
        ),
    ];

    // Real MiniLM vectors when possible so "Search (HNSW + exact rerank)" ranks by meaning.
    // Falls back to zero vectors if the local model is unavailable (offline first run).
    let demo_vectors: Option<Vec<Vec<f32>>> = if with_ann {
        let hub = crate::embeddings::EmbeddingHub::default();
        match hub.ensure_local_default() {
            Ok(()) => {
                let texts: Vec<String> = samples
                    .iter()
                    .map(|(_, _, _, body, _, _)| (*body).to_string())
                    .collect();
                hub.embed(&texts, None).ok().map(|r| r.vectors)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    db.transaction(|txn| {
        txn.put("tenants", vec![(1, Value::Int64(1)), (2, bytes("acme")), (3, bytes("pro"))])?;
        txn.put(
            "tenants",
            vec![(1, Value::Int64(2)), (2, bytes("globex")), (3, bytes("free"))],
        )?;

        for (id, tenant, name, role) in [
            (1i64, 1i64, "Ada", "admin"),
            (2, 1, "Grace", "editor"),
            (3, 2, "Alan", "editor"),
            (4, 2, "Katherine", "viewer"),
        ] {
            txn.put(
                "authors",
                vec![
                    (1, Value::Int64(id)),
                    (2, Value::Int64(tenant)),
                    (3, bytes(name)),
                    (4, bytes(role)),
                ],
            )?;
        }

        for (i, (id, tenant, author, body, status, score)) in samples.into_iter().enumerate() {
            let mut row = vec![
                (1, Value::Int64(id)),
                (2, Value::Int64(tenant)),
                (3, Value::Int64(author)),
                (4, bytes(body)),
                (5, bytes(status)),
                (6, Value::Float64(score)),
            ];
            if with_ann {
                let emb = demo_vectors
                    .as_ref()
                    .and_then(|vs| vs.get(i))
                    .cloned()
                    .unwrap_or_else(|| vec![0.0; 384]);
                row.push((7, Value::Embedding(emb)));
            }
            txn.put("documents", row)?;
        }

        let now = chrono::Utc::now().timestamp_millis();
        for i in 0..16 {
            let doc_id = (i % 8) + 1;
            let tenant_id = if doc_id <= 3 || doc_id == 7 { 1 } else { 2 };
            let kind = match i % 4 {
                0 => "ingest",
                1 => "query",
                2 => "compact",
                _ => "export",
            };
            txn.put(
                "events",
                vec![
                    (1, Value::Int64(i + 1)),
                    (2, Value::Int64(doc_id)),
                    (3, Value::Int64(tenant_id)),
                    (4, bytes(kind)),
                    (
                        5,
                        Value::Json(
                            format!(r#"{{"n":{i},"document_id":{doc_id},"source":"demo"}}"#)
                                .into_bytes(),
                        ),
                    ),
                    (6, Value::Int64(now - (i * 45_000))),
                ],
            )?;
        }

        for (id, name) in [
            (1i64, "ai"),
            (2, "storage"),
            (3, "sql"),
            (4, "ops"),
            (5, "retrieval"),
        ] {
            txn.put("tags", vec![(1, Value::Int64(id)), (2, bytes(name))])?;
        }

        // document ↔ tag links (many-to-many)
        let links = [
            (1i64, 1i64, 1i64),
            (2, 1, 5),
            (3, 2, 1),
            (4, 2, 2),
            (5, 3, 1),
            (6, 4, 3),
            (7, 5, 2),
            (8, 6, 5),
            (9, 7, 1),
            (10, 8, 4),
            (11, 8, 5),
            (12, 4, 5),
        ];
        for (id, doc, tag) in links {
            txn.put(
                "document_tags",
                vec![
                    (1, Value::Int64(id)),
                    (2, Value::Int64(doc)),
                    (3, Value::Int64(tag)),
                ],
            )?;
        }
        Ok(())
    })?;

    Ok(())
}

/// Persist a Kit schema sidecar derived from the live core catalog so Kit
/// clients can open this root. Application tables only (skip `__kit_*`).
fn write_kit_schema_sidecar(path: &Path, db: &Database) -> AppResult<()> {
    let schema = kit_schema_from_core_catalog(db).map_err(AppError::db)?;
    let json = serde_json::to_string_pretty(&schema)
        .map_err(|e| AppError::msg(format!("serialize kit_schema.json: {e}")))?;
    std::fs::write(path.join(KIT_SCHEMA_FILE), json)
        .map_err(|e| AppError::msg(format!("write kit_schema.json: {e}")))?;
    Ok(())
}

fn kit_schema_from_core_catalog(db: &Database) -> Result<KitSchema, mongreldb_core::MongrelError> {
    let catalog = db.catalog_snapshot();
    // Keep a full slice for FK parent column-name resolution while we build.
    let catalog_tables = catalog.tables;
    let mut tables = Vec::new();
    for entry in &catalog_tables {
        if entry.name.starts_with("__kit_") {
            continue;
        }
        let mut columns = Vec::new();
        let mut primary_key = Vec::new();
        for col in &entry.schema.columns {
            let (storage, emb_dim) = core_type_to_kit(&col.ty);
            let is_pk = col.flags.contains(ColumnFlags::PRIMARY_KEY);
            let nullable = col.flags.contains(ColumnFlags::NULLABLE) || !is_pk;
            let mut kit_col = KitColumn::new(col.id as u32, col.name.clone(), storage);
            kit_col.nullable = nullable && !is_pk;
            kit_col.primary_key = is_pk;
            kit_col.embedding_dim = emb_dim;
            if is_pk {
                kit_col.nullable = false;
                primary_key.push(col.name.clone());
            }
            columns.push(kit_col);
        }

        let col_name = |id: u16| -> Option<String> {
            entry
                .schema
                .columns
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
        };

        let mut indexes = Vec::new();
        for idx in &entry.schema.indexes {
            let Some(col_name) = col_name(idx.column_id) else {
                continue;
            };
            indexes.push(kit_index_from_core(idx, col_name));
        }

        let mut foreign_keys = Vec::new();
        for fk in &entry.schema.constraints.foreign_keys {
            let cols: Vec<String> = fk
                .columns
                .iter()
                .filter_map(|id| col_name(*id))
                .collect();
            // Resolve referenced column names from the catalog entry for that table.
            let ref_cols: Vec<String> = catalog_tables
                .iter()
                .find(|t| t.name == fk.ref_table)
                .map(|parent| {
                    fk.ref_columns
                        .iter()
                        .filter_map(|id| {
                            parent
                                .schema
                                .columns
                                .iter()
                                .find(|c| c.id == *id)
                                .map(|c| c.name.clone())
                        })
                        .collect()
                })
                .unwrap_or_default();
            if cols.is_empty() || ref_cols.is_empty() {
                continue;
            }
            foreign_keys.push(KitForeignKey {
                name: fk.name.clone(),
                columns: cols,
                references_table: fk.ref_table.clone(),
                references_columns: ref_cols,
                on_delete: KitFkAction::Restrict,
            });
        }

        tables.push(KitTable {
            id: entry.table_id as u32,
            name: entry.name.clone(),
            columns,
            primary_key,
            indexes,
            foreign_keys,
            unique_constraints: vec![],
            check_constraints: vec![],
        });
    }
    KitSchema::new(tables).map_err(|e| mongreldb_core::MongrelError::Schema(e.to_string()))
}

fn core_type_to_kit(ty: &TypeId) -> (KitColumnType, Option<u32>) {
    match ty {
        TypeId::Bool => (KitColumnType::Bool, None),
        TypeId::Int8 => (KitColumnType::Int8, None),
        TypeId::Int16 => (KitColumnType::Int16, None),
        TypeId::Int32 => (KitColumnType::Int32, None),
        TypeId::Int64 => (KitColumnType::Int64, None),
        TypeId::UInt8 | TypeId::UInt16 | TypeId::UInt32 | TypeId::UInt64 => {
            // Kit has no unsigned variants; store as signed peer of same width.
            match ty {
                TypeId::UInt8 => (KitColumnType::Int8, None),
                TypeId::UInt16 => (KitColumnType::Int16, None),
                TypeId::UInt32 => (KitColumnType::Int32, None),
                _ => (KitColumnType::Int64, None),
            }
        }
        TypeId::Float32 => (KitColumnType::Float32, None),
        TypeId::Float64 => (KitColumnType::Float64, None),
        // Demo + Viewer treat UTF-8 strings as TypeId::Bytes; Kit's Text is the
        // application-facing equivalent (still Bytes on the wire).
        TypeId::Bytes => (KitColumnType::Text, None),
        TypeId::Json => (KitColumnType::Json, None),
        TypeId::Date32 => (KitColumnType::Date, None),
        TypeId::Date64 => (KitColumnType::Date64, None),
        TypeId::TimestampNanos => (KitColumnType::TimestampNanos, None),
        TypeId::Time64 => (KitColumnType::Time64, None),
        TypeId::Interval => (KitColumnType::Interval, None),
        TypeId::Uuid => (KitColumnType::Uuid, None),
        TypeId::Decimal128 { .. } => (KitColumnType::Decimal128, None),
        TypeId::Array { .. } => (KitColumnType::Array, None),
        TypeId::Embedding { dim } => (KitColumnType::Embedding, Some(*dim)),
        TypeId::Enum { .. } => (KitColumnType::Text, None),
    }
}

fn kit_index_from_core(idx: &IndexDef, column_name: String) -> KitIndex {
    let kind = match idx.kind {
        CoreIndexKind::Bitmap => KitIndexKind::Bitmap,
        CoreIndexKind::FmIndex => KitIndexKind::Fm,
        CoreIndexKind::Ann => KitIndexKind::Ann,
        CoreIndexKind::LearnedRange => KitIndexKind::LearnedRange,
        CoreIndexKind::MinHash => KitIndexKind::MinHash,
        CoreIndexKind::Sparse => KitIndexKind::Sparse,
    };
    let mut kit = KitIndex {
        name: idx.name.clone(),
        columns: vec![column_name],
        unique: false,
        kind,
        ..Default::default()
    };
    if let Some(ann) = &idx.options.ann {
        kit.ann_quantization = match ann.quantization {
            CoreAnnQuantization::BinarySign => KitAnnQuantization::BinarySign,
            CoreAnnQuantization::Dense => KitAnnQuantization::Dense,
            CoreAnnQuantization::Product {
                num_subvectors,
                bits,
            } => KitAnnQuantization::Product {
                num_subvectors,
                bits,
            },
        };
        kit.ann_algorithm = match ann.algorithm {
            CoreAnnAlgorithm::Hnsw => KitAnnAlgorithm::Hnsw,
            CoreAnnAlgorithm::DiskAnn => KitAnnAlgorithm::DiskAnn,
            CoreAnnAlgorithm::Ivf => KitAnnAlgorithm::Ivf,
        };
        kit.ann_m = Some(ann.m);
        kit.ann_ef_construction = Some(ann.ef_construction);
        kit.ann_ef_search = Some(ann.ef_search);
        if let Some(diskann) = &ann.diskann {
            kit.ann_diskann_r = Some(diskann.r);
            kit.ann_diskann_l = Some(diskann.l);
            kit.ann_diskann_beam_width = Some(diskann.beam_width);
            kit.ann_diskann_alpha = Some(diskann.alpha);
        }
        if let Some(ivf) = &ann.ivf {
            kit.ann_ivf_nlist = Some(ivf.nlist);
            kit.ann_ivf_nprobe = Some(ivf.nprobe);
            kit.ann_ivf_training_samples = Some(ivf.training_samples);
        }
        if let Some(product) = &ann.product {
            kit.ann_pq_training_samples = Some(product.training_samples);
            kit.ann_pq_seed = Some(product.seed);
            kit.ann_pq_rerank_factor = Some(product.rerank_factor);
        }
    }
    if let Some(minhash) = &idx.options.minhash {
        kit.minhash_permutations = Some(minhash.permutations);
    }
    kit
}

/// True if `path` already contains MongrelDB storage markers.
fn looks_like_mongreldb(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    path.join("CATALOG").is_file()
        || path.join("_meta").is_dir()
        || path.join("tables").is_dir()
        || path.join("_wal").is_dir()
}

fn open_existing(
    path: &Path,
    username: Option<&str>,
    password: Option<&str>,
    passphrase: Option<&str>,
) -> AppResult<Database> {
    match (passphrase, username, password) {
        (Some(pass), Some(user), Some(pw)) => {
            Database::open_encrypted_with_credentials(path, pass, user, pw).map_err(AppError::db)
        }
        (Some(pass), None, None) => Database::open_encrypted(path, pass).map_err(AppError::db),
        (None, Some(user), Some(pw)) => {
            Database::open_with_credentials(path, user, pw).map_err(AppError::db)
        }
        (None, None, None) => Database::open(path).map_err(AppError::db),
        _ => Err(AppError::msg("invalid credential combination")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_demo_dir() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "mongreldb-viewer-demo-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn create_demo_writes_kit_schema_openable_by_kit() {
        let path = unique_demo_dir();
        // with_ann=false avoids embedding model download in CI/offline.
        let session = DbSession::create_demo(&path, false).expect("create demo");
        drop(session);

        assert!(
            path.join(KIT_SCHEMA_FILE).is_file(),
            "demo root must include {KIT_SCHEMA_FILE} for Kit/Mongrel clients"
        );

        let kit = mongreldb_kit::Database::open(&path).expect("kit open demo root");
        let mut names = kit.table_names();
        names.sort();
        assert_eq!(
            names,
            vec![
                "authors".to_string(),
                "document_tags".to_string(),
                "documents".to_string(),
                "events".to_string(),
                "tags".to_string(),
                "tenants".to_string(),
            ]
        );
        let rows = kit
            .sql_rows("SELECT count(*) AS c FROM documents")
            .expect("count documents");
        assert_eq!(rows[0].get("c").and_then(|v| v.as_i64()), Some(8));

        let _ = std::fs::remove_dir_all(&path);
    }
}
