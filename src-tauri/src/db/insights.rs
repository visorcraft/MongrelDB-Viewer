//! Overview insights + suggested SQL derived from each table’s schema
//! (columns, types, flags, and secondary indexes). No demo-table hardcoding.

use crate::db::connection::Connection;
use crate::error::AppResult;
use crate::models::{
    ColumnInfo, DbInsights, IndexInfo, InsightCard, SuggestedQuery, TableDetail, TableSummary,
};

const MAX_SUGGESTIONS: usize = 28;
const MAX_PROBE_GROUP_BYS: usize = 4;

pub async fn build_insights(conn: &Connection) -> AppResult<DbInsights> {
    let overview = conn.overview()?;
    let mut cards = Vec::new();
    let mut suggested = Vec::new();

    let total_rows: u64 = overview.tables.iter().map(|t| t.row_count).sum();
    let ann_tables = overview.tables.iter().filter(|t| t.has_ann).count();
    let indexed = overview
        .tables
        .iter()
        .map(|t| t.index_count)
        .sum::<usize>();

    cards.push(InsightCard {
        title: "Tables".into(),
        value: overview.table_count.to_string(),
        detail: format!("{total_rows} rows total"),
        accent: "cyan".into(),
        sql: Some("SELECT name FROM information_schema.tables ORDER BY name".into()),
    });
    cards.push(InsightCard {
        title: "Secondary indexes".into(),
        value: indexed.to_string(),
        detail: "Bitmap · Range · Text · ANN · Sparse · MinHash".into(),
        accent: "violet".into(),
        sql: None,
    });
    cards.push(InsightCard {
        title: "Vector-ready".into(),
        value: ann_tables.to_string(),
        detail: if ann_tables > 0 {
            "HNSW ANN present - try Vector search".into()
        } else {
            "No ANN yet - open Vector search to install".into()
        },
        accent: "magenta".into(),
        sql: None,
    });

    suggested.push(SuggestedQuery {
        title: "List tables".into(),
        description: "Catalog via information_schema".into(),
        sql: "SELECT name FROM information_schema.tables ORDER BY name".into(),
        category: "catalog".into(),
    });

    let mut probes_done = 0usize;

    for t in &overview.tables {
        let detail = match conn.table_detail(&t.name) {
            Ok(d) => d,
            Err(_) => {
                // Still offer generic browse/count without schema.
                push_basic(&mut cards, &mut suggested, t);
                continue;
            }
        };

        push_basic(&mut cards, &mut suggested, t);
        push_schema_recipes(&mut suggested, &detail);

        // Live insight cards: group-by first good categorical column(s).
        if probes_done < MAX_PROBE_GROUP_BYS {
            if let Some(col) = first_groupable(&detail) {
                let sql = format!(
                    "SELECT {col}, count(*) AS n FROM {} GROUP BY {col} ORDER BY n DESC LIMIT 20",
                    detail.name
                );
                let title = format!("{} by {col}", detail.name);
                let accent = if index_kind_on(&detail, &col).map(|k| k.contains("bitmap")).unwrap_or(false)
                {
                    "magenta"
                } else {
                    "amber"
                };
                if probe(conn, &sql, &mut cards, &title, accent)
                    .await
                    .is_ok()
                {
                    probes_done += 1;
                }
            }
        }
    }

    // De-dupe by SQL; keep order.
    let mut seen = std::collections::HashSet::new();
    suggested.retain(|q| seen.insert(q.sql.clone()));
    if suggested.len() > MAX_SUGGESTIONS {
        suggested.truncate(MAX_SUGGESTIONS);
    }

    Ok(DbInsights {
        cards,
        suggested_queries: suggested,
        connection_mode: overview.connection_mode,
        label: overview.display_label,
    })
}

fn push_basic(cards: &mut Vec<InsightCard>, suggested: &mut Vec<SuggestedQuery>, t: &TableSummary) {
    let name = &t.name;
    cards.push(InsightCard {
        title: name.clone(),
        value: t.row_count.to_string(),
        detail: format!(
            "{} cols · {} indexes{}",
            t.column_count,
            t.index_count,
            if t.has_ann { " · ANN" } else { "" }
        ),
        accent: if t.has_ann { "magenta" } else { "cyan" }.into(),
        sql: Some(format!("SELECT * FROM {name} LIMIT 25")),
    });
    suggested.push(SuggestedQuery {
        title: format!("Browse {name}"),
        description: "First 25 rows".into(),
        sql: format!("SELECT * FROM {name} LIMIT 25"),
        category: "browse".into(),
    });
    suggested.push(SuggestedQuery {
        title: format!("Count {name}"),
        description: "Exact row count".into(),
        sql: format!("SELECT count(*) AS n FROM {name}"),
        category: "stats".into(),
    });
}

fn push_schema_recipes(suggested: &mut Vec<SuggestedQuery>, detail: &TableDetail) {
    let table = &detail.name;
    let project = projection_cols(detail);
    let project_sql = if project.is_empty() {
        "*".to_string()
    } else {
        project.join(", ")
    };

    // Prefer non-embedding projection for everyday browsing.
    if project_sql != "*" {
        suggested.push(SuggestedQuery {
            title: format!("Browse {table} (no vectors)"),
            description: "Skip embedding columns".into(),
            sql: format!("SELECT {project_sql} FROM {table} LIMIT 25"),
            category: "browse".into(),
        });
    }

    // GROUP BY categorical / bitmap columns.
    for col in groupable_columns(detail).into_iter().take(3) {
        let idx_note = index_kind_on(detail, &col)
            .map(|k| format!("uses {k} index"))
            .unwrap_or_else(|| "cardinality sample".into());
        suggested.push(SuggestedQuery {
            title: format!("{table} by {col}"),
            description: idx_note,
            sql: format!(
                "SELECT {col}, count(*) AS n FROM {table} GROUP BY {col} ORDER BY n DESC LIMIT 20"
            ),
            category: "stats".into(),
        });
    }

    // Numeric range / sort.
    for col in numeric_columns(detail).into_iter().take(2) {
        suggested.push(SuggestedQuery {
            title: format!("Top {table} by {col}"),
            description: "Descending sort".into(),
            sql: format!(
                "SELECT {project_sql} FROM {table} ORDER BY {col} DESC NULLS LAST LIMIT 20"
            ),
            category: "filter".into(),
        });
        // Soft threshold when column name looks like a score/ratio.
        if looks_like_score(&col) {
            suggested.push(SuggestedQuery {
                title: format!("High {col} in {table}"),
                description: "Filter {col} ≥ 0.9".replace("{col}", &col),
                sql: format!(
                    "SELECT {project_sql} FROM {table} WHERE {col} >= 0.9 ORDER BY {col} DESC LIMIT 20"
                ),
                category: "filter".into(),
            });
        }
    }

    // Time-ish order.
    if let Some(col) = temporal_column(detail) {
        suggested.push(SuggestedQuery {
            title: format!("Recent {table}"),
            description: format!("Newest by {col}"),
            sql: format!(
                "SELECT {project_sql} FROM {table} ORDER BY {col} DESC NULLS LAST LIMIT 25"
            ),
            category: "browse".into(),
        });
    }

    // Text / FM columns → LIKE sample (safe placeholder pattern).
    for col in text_columns(detail).into_iter().take(2) {
        let has_fm = index_kind_on(detail, &col)
            .map(|k| k.contains("fm") || k.contains("text"))
            .unwrap_or(false);
        suggested.push(SuggestedQuery {
            title: format!("Search {table}.{col}"),
            description: if has_fm {
                "Substring filter (FM-friendly)".into()
            } else {
                "Substring filter (edit the pattern)".into()
            },
            sql: format!(
                "SELECT {project_sql} FROM {table} WHERE cast({col} AS varchar) LIKE '%a%' LIMIT 20"
            ),
            category: "search".into(),
        });
    }

    // Equality filter template on first categorical column (user edits the literal).
    if let Some(cat) = first_groupable(detail) {
        suggested.push(SuggestedQuery {
            title: format!("Filter {table} on {cat}"),
            description: format!("Edit the literal after WHERE {cat} = …"),
            sql: format!(
                "SELECT {project_sql} FROM {table} WHERE cast({cat} AS varchar) = 'active' LIMIT 20"
            ),
            category: "filter".into(),
        });
    }

    // ANN present → nudge toward vector UI, not fake vectors.
    if detail.index_radar.ann > 0 || detail.columns.iter().any(|c| c.embedding_dim.is_some()) {
        if let Some(emb) = detail
            .columns
            .iter()
            .find(|c| c.embedding_dim.is_some())
            .map(|c| c.name.clone())
        {
            suggested.push(SuggestedQuery {
                title: format!("List {table} with {emb}"),
                description: "Embeddings present - use Vector search for k-NN".into(),
                sql: format!("SELECT {project_sql} FROM {table} LIMIT 10"),
                category: "search".into(),
            });
        }
    }

    // Null checks on nullable columns.
    if let Some(col) = detail
        .columns
        .iter()
        .find(|c| c.flags.iter().any(|f| f == "NULLABLE") && c.embedding_dim.is_none())
        .map(|c| c.name.clone())
    {
        suggested.push(SuggestedQuery {
            title: format!("Non-null {table}.{col}"),
            description: "Rows where column is present".into(),
            sql: format!(
                "SELECT {project_sql} FROM {table} WHERE {col} IS NOT NULL LIMIT 25"
            ),
            category: "filter".into(),
        });
    }
}

fn projection_cols(detail: &TableDetail) -> Vec<String> {
    detail
        .columns
        .iter()
        .filter(|c| c.embedding_dim.is_none())
        .filter(|c| !is_embedding_type(&c.type_name))
        .map(|c| c.name.clone())
        .take(8)
        .collect()
}

fn groupable_columns(detail: &TableDetail) -> Vec<String> {
    let mut out = Vec::new();

    // 1) Bitmap-indexed columns first (equality / low-cardinality sweet spot).
    for idx in &detail.indexes {
        if kind_is(idx, "bitmap") {
            push_unique(&mut out, &idx.column_name);
        }
    }

    // 2) Columns that look categorical by name.
    for c in &detail.columns {
        if c.embedding_dim.is_some() || is_embedding_type(&c.type_name) {
            continue;
        }
        if is_primary_key(c) {
            continue;
        }
        if looks_categorical(&c.name) && is_textish(&c.type_name) {
            push_unique(&mut out, &c.name);
        }
    }

    // 3) Any remaining short text-ish non-PK columns (Bytes/Bool).
    for c in &detail.columns {
        if c.embedding_dim.is_some() || is_embedding_type(&c.type_name) || is_primary_key(c) {
            continue;
        }
        if is_bool_type(&c.type_name) || (is_textish(&c.type_name) && c.name.len() <= 24) {
            push_unique(&mut out, &c.name);
        }
    }

    out
}

fn first_groupable(detail: &TableDetail) -> Option<String> {
    groupable_columns(detail).into_iter().next()
}

fn numeric_columns(detail: &TableDetail) -> Vec<String> {
    let mut out = Vec::new();
    // Prefer learned-range indexed numerics.
    for idx in &detail.indexes {
        if kind_is(idx, "learned") || kind_is(idx, "range") || kind_is(idx, "pgm") {
            if let Some(c) = detail.columns.iter().find(|c| c.name == idx.column_name) {
                if is_numeric_type(&c.type_name) {
                    push_unique(&mut out, &c.name);
                }
            }
        }
    }
    for c in &detail.columns {
        if is_primary_key(c) {
            continue;
        }
        if is_numeric_type(&c.type_name) && !looks_like_id(&c.name) {
            push_unique(&mut out, &c.name);
        }
    }
    out
}

fn text_columns(detail: &TableDetail) -> Vec<String> {
    let mut out = Vec::new();
    for idx in &detail.indexes {
        if kind_is(idx, "fm") {
            push_unique(&mut out, &idx.column_name);
        }
    }
    for c in &detail.columns {
        if c.embedding_dim.is_some() || is_embedding_type(&c.type_name) || is_primary_key(c) {
            continue;
        }
        if is_textish(&c.type_name)
            && (looks_like_text_content(&c.name) || index_kind_on(detail, &c.name).is_some())
        {
            push_unique(&mut out, &c.name);
        }
    }
    out
}

fn temporal_column(detail: &TableDetail) -> Option<String> {
    detail
        .columns
        .iter()
        .find(|c| looks_temporal(&c.name) || is_temporal_type(&c.type_name))
        .map(|c| c.name.clone())
        .or_else(|| {
            numeric_columns(detail)
                .into_iter()
                .find(|n| looks_temporal(n))
        })
}

fn index_kind_on(detail: &TableDetail, col: &str) -> Option<String> {
    detail
        .indexes
        .iter()
        .find(|i| i.column_name == col)
        .map(|i| i.kind.clone())
}

fn kind_is(idx: &IndexInfo, needle: &str) -> bool {
    idx.kind.to_ascii_lowercase().contains(needle)
}

fn is_primary_key(c: &ColumnInfo) -> bool {
    c.flags.iter().any(|f| f == "PRIMARY_KEY")
}

fn is_embedding_type(ty: &str) -> bool {
    ty.to_ascii_lowercase().contains("embedding")
}

fn is_numeric_type(ty: &str) -> bool {
    let t = ty.to_ascii_lowercase();
    t.contains("int")
        || t.contains("float")
        || t.contains("double")
        || t.contains("decimal")
        || t.contains("numeric")
}

fn is_textish(ty: &str) -> bool {
    let t = ty.to_ascii_lowercase();
    t.contains("bytes")
        || t.contains("utf")
        || t.contains("string")
        || t.contains("varchar")
        || t.contains("text")
        || t.contains("json")
        || t.contains("enum")
}

fn is_bool_type(ty: &str) -> bool {
    ty.to_ascii_lowercase().contains("bool")
}

fn is_temporal_type(ty: &str) -> bool {
    let t = ty.to_ascii_lowercase();
    t.contains("timestamp") || t.contains("date") || t.contains("time")
}

fn looks_categorical(name: &str) -> bool {
    const KEYS: &[&str] = &[
        "status",
        "state",
        "kind",
        "type",
        "category",
        "class",
        "role",
        "level",
        "tier",
        "region",
        "country",
        "city",
        "lang",
        "locale",
        "source",
        "channel",
        "priority",
        "flag",
        "mode",
        "phase",
        "stage",
        "tenant",
        "org",
        "gender",
        "currency",
        "provider",
    ];
    let n = name.to_ascii_lowercase();
    KEYS.iter().any(|k| n == *k || n.ends_with(&format!("_{k}")) || n.starts_with(&format!("{k}_")))
}

fn looks_like_score(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("score")
        || n.contains("rating")
        || n.contains("ratio")
        || n.contains("prob")
        || n.contains("confidence")
        || n == "weight"
}

fn looks_like_id(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "id" || n.ends_with("_id") || n.ends_with("id") && n.len() <= 4
}

fn looks_like_text_content(name: &str) -> bool {
    const KEYS: &[&str] = &[
        "body", "text", "content", "title", "name", "description", "desc", "message",
        "msg", "comment", "note", "summary", "label", "subject", "query", "prompt",
        "path", "url", "email",
    ];
    let n = name.to_ascii_lowercase();
    KEYS.iter().any(|k| n == *k || n.contains(k))
}

fn looks_temporal(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "ts"
        || n == "time"
        || n.contains("time")
        || n.contains("date")
        || n.contains("created")
        || n.contains("updated")
        || n.contains("timestamp")
        || n.ends_with("_at")
        || n.ends_with("_on")
}

fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|x| x == name) {
        out.push(name.to_string());
    }
}

async fn probe(
    conn: &Connection,
    sql: &str,
    cards: &mut Vec<InsightCard>,
    title: &str,
    accent: &str,
) -> AppResult<()> {
    let result = conn
        .run_sql(crate::models::SqlRequest {
            sql: sql.to_string(),
            max_rows: Some(12),
        })
        .await?;
    if result.rows.is_empty() {
        return Ok(());
    }
    let mut parts = Vec::new();
    for row in result.rows.iter().take(4) {
        if row.len() >= 2 {
            parts.push(format!("{}={}", cell(&row[0]), cell(&row[1])));
        }
    }
    let detail = if parts.is_empty() {
        format!("{} groups", result.row_count)
    } else {
        parts.join(" · ")
    };
    cards.push(InsightCard {
        title: title.into(),
        value: result.row_count.to_string(),
        detail,
        accent: accent.into(),
        sql: Some(sql.to_string()),
    });
    Ok(())
}

fn cell(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "∅".into(),
        other => other.to_string(),
    }
}
