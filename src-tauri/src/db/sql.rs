use std::time::Instant;

use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;

use std::sync::Arc;

use mongreldb_query::MongrelSession;

use crate::db::session::DbSession;
use crate::error::{AppError, AppResult};
use crate::models::{ReindexRequest, ReindexResult, SqlRequest, SqlResult};

pub async fn run_sql(db: &DbSession, req: SqlRequest) -> AppResult<SqlResult> {
    run_sql_session(Arc::clone(&db.session), req).await
}

/// Engine maintenance: `REINDEX` or `REINDEX <table>`.
///
/// Runs analyze + compact on the target table(s), then GC. Works on Direct
/// sessions; callers that use server SQL should send the same statement via
/// `sql_work` when the server supports it.
pub async fn reindex(db: &DbSession, req: ReindexRequest) -> AppResult<ReindexResult> {
    reindex_session(Arc::clone(&db.session), req).await
}

pub async fn reindex_session(
    session: Arc<MongrelSession>,
    req: ReindexRequest,
) -> AppResult<ReindexResult> {
    let table = req
        .table
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (sql, target) = match table {
        None => ("REINDEX".to_string(), "database".to_string()),
        Some(name) => {
            let ident = sanitize_sql_ident(name)?;
            (format!("REINDEX {ident}"), ident)
        }
    };
    let started = Instant::now();
    session
        .run(&sql)
        .await
        .map_err(|e| AppError::sql(format!("REINDEX failed: {e}")))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let message = if target == "database" {
        format!("REINDEX completed for the entire database in {elapsed_ms} ms (analyze + compact + GC).")
    } else {
        format!(
            "REINDEX completed for table `{target}` in {elapsed_ms} ms (analyze + compact + GC)."
        )
    };
    Ok(ReindexResult {
        target,
        message,
        elapsed_ms,
    })
}

/// Allow only simple SQL identifiers (table/index names without quoting).
fn sanitize_sql_ident(name: &str) -> AppResult<String> {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(AppError::msg(format!(
            "invalid identifier {name:?}: use letters, digits, and underscore only"
        )));
    }
    Ok(name.to_string())
}

pub async fn run_sql_session(session: Arc<MongrelSession>, req: SqlRequest) -> AppResult<SqlResult> {
    let sql = req.sql.trim();
    if sql.is_empty() {
        return Err(AppError::sql("empty SQL"));
    }
    let max_rows = req.max_rows.unwrap_or(500).clamp(1, 10_000);
    let started = Instant::now();
    let kind = classify(sql);

    let batches = session.run(sql).await.map_err(AppError::sql)?;
    let (columns, mut rows) = batches_to_rows(&batches)?;
    let total = rows.len();
    let truncated = total > max_rows;
    if truncated {
        rows.truncate(max_rows);
    }

    Ok(SqlResult {
        columns,
        rows,
        row_count: total.min(max_rows),
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
        statement_kind: kind,
    })
}

fn classify(sql: &str) -> String {
    let head = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    match head.as_str() {
        "SELECT" | "WITH" | "EXPLAIN" | "SHOW" | "DESCRIBE" | "DESC" | "VALUES" => "query".into(),
        "INSERT" | "UPDATE" | "DELETE" | "MERGE" | "TRUNCATE" => "dml".into(),
        "CREATE" | "ALTER" | "DROP" | "RENAME" => "ddl".into(),
        "REINDEX" | "ANALYZE" | "VACUUM" | "OPTIMIZE" => "maintenance".into(),
        "BEGIN" | "COMMIT" | "ROLLBACK" | "SAVEPOINT" => "txn".into(),
        "ATTACH" | "DETACH" | "PRAGMA" | "SET" | "USE" => "session".into(),
        other if other.is_empty() => "empty".into(),
        other => other.to_ascii_lowercase(),
    }
}

pub fn batches_to_rows_public(
    batches: &[RecordBatch],
) -> AppResult<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
    batches_to_rows(batches)
}

fn batches_to_rows(
    batches: &[RecordBatch],
) -> AppResult<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
    if batches.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let schema = batches[0].schema();
    let columns: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    let mut rows = Vec::new();
    for batch in batches {
        let n = batch.num_rows();
        let arrays: Vec<ArrayRef> = (0..batch.num_columns())
            .map(|i| batch.column(i).clone())
            .collect();
        for row_idx in 0..n {
            let mut row = Vec::with_capacity(arrays.len());
            for array in &arrays {
                row.push(array_value_json(array, row_idx));
            }
            rows.push(row);
        }
    }
    Ok((columns, rows))
}

fn array_value_json(array: &ArrayRef, row: usize) -> serde_json::Value {
    if array.is_null(row) {
        return serde_json::Value::Null;
    }
    use arrow::array::*;
    use arrow::datatypes::DataType;

    match array.data_type() {
        DataType::Boolean => {
            let a = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            serde_json::Value::Bool(a.value(row))
        }
        DataType::Int8 => {
            let a = array.as_any().downcast_ref::<Int8Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Int16 => {
            let a = array.as_any().downcast_ref::<Int16Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Int32 => {
            let a = array.as_any().downcast_ref::<Int32Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Int64 => {
            let a = array.as_any().downcast_ref::<Int64Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::UInt8 => {
            let a = array.as_any().downcast_ref::<UInt8Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::UInt16 => {
            let a = array.as_any().downcast_ref::<UInt16Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::UInt32 => {
            let a = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::UInt64 => {
            let a = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Float32 => {
            let a = array.as_any().downcast_ref::<Float32Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Float64 => {
            let a = array.as_any().downcast_ref::<Float64Array>().unwrap();
            serde_json::json!(a.value(row))
        }
        DataType::Utf8 => {
            let a = array.as_any().downcast_ref::<StringArray>().unwrap();
            serde_json::Value::String(a.value(row).to_string())
        }
        DataType::LargeUtf8 => {
            let a = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
            serde_json::Value::String(a.value(row).to_string())
        }
        DataType::Binary => {
            let a = array.as_any().downcast_ref::<BinaryArray>().unwrap();
            let bytes = a.value(row);
            match std::str::from_utf8(bytes) {
                Ok(s) => serde_json::Value::String(s.to_string()),
                Err(_) => serde_json::Value::String(format!("\\x{}", hex::encode_simple(bytes))),
            }
        }
        DataType::LargeBinary => {
            let a = array.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
            let bytes = a.value(row);
            match std::str::from_utf8(bytes) {
                Ok(s) => serde_json::Value::String(s.to_string()),
                Err(_) => serde_json::Value::String(format!("\\x{}", hex::encode_simple(bytes))),
            }
        }
        DataType::FixedSizeList(_, _) => {
            if let Some(list) = array.as_any().downcast_ref::<FixedSizeListArray>() {
                let values = list.value(row);
                if let Some(f) = values.as_any().downcast_ref::<Float32Array>() {
                    let v: Vec<f32> = (0..f.len()).map(|i| f.value(i)).collect();
                    // Compact preview for large embeddings
                    if v.len() > 8 {
                        let mut preview = Vec::with_capacity(8);
                        preview.extend_from_slice(&v[..4]);
                        preview.extend_from_slice(&v[v.len() - 4..]);
                        return serde_json::json!({
                            "dim": v.len(),
                            "preview": preview,
                        });
                    }
                    return serde_json::json!(v);
                }
            }
            serde_json::Value::String(format!("{:?}", array.data_type()))
        }
        other => {
            // Fallback: try string cast via Debug-ish display
            let _ = other;
            if let Some(a) = array.as_any().downcast_ref::<StringArray>() {
                return serde_json::Value::String(a.value(row).to_string());
            }
            serde_json::Value::String(format!("<{other:?}>"))
        }
    }
}

/// Tiny hex helper so we don't need the hex crate.
mod hex {
    pub fn encode_simple(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for b in bytes.iter().take(64) {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0xf) as usize] as char);
        }
        if bytes.len() > 64 {
            out.push('…');
        }
        out
    }
}
