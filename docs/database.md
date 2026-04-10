# Database Conventions

This document defines patterns for SQL schema design and the corresponding Rust DB structs. Follow these conventions when adding Groupshop domains such as products, deals, commitments, users, and fulfillment.

## Migration File Organization

Each domain gets its own migration file in `cloudflare/api/db/migrations/`:

| File | Domain |
|---|---|
| `0001_user_auth.sql` | Auth tables + role seed data |
| `0002_user_profile.sql` | User profile |
| `0003_product_catalog.sql` | Product catalog |
| `0004_deal_catalog.sql` | Deal catalog and deal state |
| `0005_order_fulfillment.sql` | Order and fulfillment state |

When adding a new domain, create a new migration file. Keep one domain per file. During development, edit files in place and run `task db:recreate-dev`.

## Case-Insensitive Uniqueness

Use `COLLATE NOCASE` on the column definition — do **not** create a separate `*_normalized` column.

```sql
CREATE TABLE product_catalog (
    base_name TEXT NOT NULL COLLATE NOCASE,
    ...
);

CREATE UNIQUE INDEX idx_product_catalog_base_name ON product_catalog(base_name);
```

This pattern is useful for:

- `user_account_profile.username`
- `product_catalog.base_name`
- `deal_catalog.slug`

## Slug Validation

Slugs (`base_name`, `slug`) must be lowercase alphanumeric with hyphens. Enforce via SQL triggers:

```sql
SELECT CASE
    WHEN trim(NEW.base_name) GLOB '*[^a-z0-9-]*' THEN
        RAISE(ABORT, 'base_name characters invalid')
END;
SELECT CASE
    WHEN NEW.base_name != lower(trim(NEW.base_name)) THEN
        RAISE(ABORT, 'base_name must be lowercase')
END;
```

## Trigger Structure

Every table with user-writable columns should have paired insert and update guard triggers:

- `trg_{table}_insert_guard` — validates all invariants on INSERT
- `trg_{table}_update_guard` — re-checks the same invariants on UPDATE of mutable columns

Both triggers should enforce the same rules. The update trigger should list the specific mutable columns in its `BEFORE UPDATE OF` clause.

## Table Conventions

- Catalog tables may use `TEXT PRIMARY KEY` and `WITHOUT ROWID`
- Include `created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP` and `updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP` on mutable tables
- Boolean columns use `INTEGER NOT NULL DEFAULT 0` (D1/SQLite has no native boolean)
- Enum-like columns use `TEXT NOT NULL` with trigger-enforced `IN (...)` checks
- Foreign keys use `FOREIGN KEY (col) REFERENCES other_table(id)` with `ON DELETE CASCADE` only for dependent rows that should be removed when the parent is deleted

## Rust DB Struct Pattern

### Naming

- Struct: `{Table}Db` — e.g. `ProductCatalogDb` for `product_catalog`
- Module: `db/{domain}/` — e.g. `db/product/catalog.rs`
- Table constant: `SQL_TABLE_{SCREAMING_SNAKE}` in `db/tables.rs`

### ID Type Safety

**All ID fields in DB structs, API request/response structs, and DB method signatures must use the real typed ID** (`ProductId`, `DealId`, `UserId`, etc.) — never raw `String` or `&str`.

**DB struct fields**: Use the typed ID directly.

```rust
#[groupshop]
pub struct ProductCatalogDb {
    pub id: ProductId,
    pub display_name: String,
}
```

**DB method signatures**: Accept typed ID references. Convert to `&str` only at the SQL binding boundary inside the method body.

```rust
pub async fn load_by_id(ctx: &ApiContext, id: &ProductId) -> ApiResult<Self> {
    db_load(db_prepare(
        &get_d1(&ctx.env)?,
        format!("SELECT {COLUMNS} FROM {SQL_TABLE_PRODUCT_CATALOG} WHERE id = ?1"),
        &[JsValue::from_str(id.as_str())],
    )?, "product not found").await
}
```

### Structure

```rust
use crate::db::tables::SQL_TABLE_PRODUCT_CATALOG;
use crate::{
    prelude::*,
    utils::{db_load, db_prepare, db_try_load, deserialize_d1_bool, get_d1},
};
use wasm_bindgen::prelude::*;

#[groupshop]
pub struct ProductCatalogDb {
    pub id: ProductId,
    pub display_name: String,
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub is_active: bool,
}
```

### Key patterns

- Use `#[groupshop]` for serde derives
- Use `#[serde(deserialize_with = "deserialize_d1_bool")]` for boolean columns
- SELECT only the columns you need, not `SELECT *`
- Use `db_prepare` + `db_load` / `db_try_load` / `db_load_all` / `db_execute` / `db_exists`
- For multi-table atomic inserts, use `prepare_insert` returning `D1PreparedStatement` + `db_batch`

### Standard methods

| Method | When |
|---|---|
| `load_by_id(ctx, id: &ProductId)` | Single row by primary key, error if missing |
| `try_load_by_id(ctx, id: &ProductId)` | Single row by primary key, `None` if missing |
| `try_load_by_{field}(ctx, value)` | Lookup by unique/indexed field |
| `load_all(ctx)` | All rows (use for small catalog tables) |
| `prepare_insert(d1, ...)` | Build INSERT for batching |
| `update_{field}(ctx, id: &ProductId, value)` | Single-field update |
| `exists(ctx, field)` | Existence check via `db_exists` |

### Unique violation detection

For columns with unique indexes, add a helper function:

```rust
pub fn is_base_name_unique_violation(db_message: &str) -> bool {
    let lower = db_message.to_ascii_lowercase();
    lower.contains("unique")
        && (lower.contains("product_catalog.base_name")
            || lower.contains("idx_product_catalog_base_name"))
}
```

## Checklist: Adding a New Domain Table

1. Create or edit the migration SQL file in `cloudflare/api/db/migrations/`
2. Use `COLLATE NOCASE` on columns that need case-insensitive uniqueness
3. Add insert + update guard triggers for all invariants
4. Run `task db:recreate-dev`
5. Add table constant in `db/tables.rs`
6. Create `db/{domain}/{table}.rs` with `{Table}Db` struct
7. Add `pub mod {domain};` to `db/mod.rs` if needed
8. Implement query methods following the standard method patterns above
9. Run `task lint`
