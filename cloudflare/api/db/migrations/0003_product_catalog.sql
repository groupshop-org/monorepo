PRAGMA foreign_keys = ON;

-- ---------------------------------------------------------------------------
-- product_category  (hierarchical via parent_id)
-- ---------------------------------------------------------------------------
CREATE TABLE product_category (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    parent_id TEXT,
    depth INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (parent_id) REFERENCES product_category(id)
) WITHOUT ROWID;

CREATE UNIQUE INDEX idx_product_category_name ON product_category(name);

DROP TRIGGER IF EXISTS trg_product_category_insert_guard;
CREATE TRIGGER trg_product_category_insert_guard
BEFORE INSERT ON product_category
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.id IS NULL OR trim(NEW.id) = '' THEN
            RAISE(ABORT, 'category id required')
    END;
    SELECT CASE
        WHEN trim(NEW.id) GLOB '*[^a-z0-9-]*' THEN
            RAISE(ABORT, 'category id characters invalid')
    END;
    SELECT CASE
        WHEN NEW.id != lower(trim(NEW.id)) THEN
            RAISE(ABORT, 'category id must be lowercase')
    END;
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'category name required')
    END;
    SELECT CASE
        WHEN NEW.depth < 0 THEN
            RAISE(ABORT, 'category depth must be >= 0')
    END;
END;

DROP TRIGGER IF EXISTS trg_product_category_update_guard;
CREATE TRIGGER trg_product_category_update_guard
BEFORE UPDATE OF name, parent_id, depth ON product_category
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'category name required')
    END;
    SELECT CASE
        WHEN NEW.depth < 0 THEN
            RAISE(ABORT, 'category depth must be >= 0')
    END;
END;

-- ---------------------------------------------------------------------------
-- product_brand
-- ---------------------------------------------------------------------------
CREATE TABLE product_brand (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
) WITHOUT ROWID;

CREATE UNIQUE INDEX idx_product_brand_name ON product_brand(name);

DROP TRIGGER IF EXISTS trg_product_brand_insert_guard;
CREATE TRIGGER trg_product_brand_insert_guard
BEFORE INSERT ON product_brand
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.id IS NULL OR trim(NEW.id) = '' THEN
            RAISE(ABORT, 'brand id required')
    END;
    SELECT CASE
        WHEN trim(NEW.id) GLOB '*[^a-z0-9-]*' THEN
            RAISE(ABORT, 'brand id characters invalid')
    END;
    SELECT CASE
        WHEN NEW.id != lower(trim(NEW.id)) THEN
            RAISE(ABORT, 'brand id must be lowercase')
    END;
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'brand name required')
    END;
END;

DROP TRIGGER IF EXISTS trg_product_brand_update_guard;
CREATE TRIGGER trg_product_brand_update_guard
BEFORE UPDATE OF name ON product_brand
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'brand name required')
    END;
END;

-- ---------------------------------------------------------------------------
-- product_catalog
-- ---------------------------------------------------------------------------
CREATE TABLE product_catalog (
    id TEXT PRIMARY KEY,
    product_hash BLOB NOT NULL,
    gtin TEXT NOT NULL,
    name TEXT NOT NULL,
    category_id TEXT NOT NULL,
    brand_id TEXT NOT NULL,
    price_cents INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    minimum_order_quantity INTEGER NOT NULL,
    inventory INTEGER NOT NULL DEFAULT 0,
    is_preorder INTEGER NOT NULL DEFAULT 0,
    estimated_delivery_weeks INTEGER,
    supplier_url TEXT NOT NULL DEFAULT '',
    image_url TEXT NOT NULL DEFAULT '',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (category_id) REFERENCES product_category(id),
    FOREIGN KEY (brand_id) REFERENCES product_brand(id)
) WITHOUT ROWID;

CREATE UNIQUE INDEX idx_product_catalog_gtin ON product_catalog(gtin);
CREATE UNIQUE INDEX idx_product_catalog_product_hash ON product_catalog(product_hash);
CREATE INDEX idx_product_catalog_category_id ON product_catalog(category_id);
CREATE INDEX idx_product_catalog_brand_id ON product_catalog(brand_id);

DROP TRIGGER IF EXISTS trg_product_catalog_insert_guard;
CREATE TRIGGER trg_product_catalog_insert_guard
BEFORE INSERT ON product_catalog
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.id IS NULL OR trim(NEW.id) = '' THEN
            RAISE(ABORT, 'product id required')
    END;
    SELECT CASE
        WHEN trim(NEW.id) GLOB '*[^a-z0-9-]*' THEN
            RAISE(ABORT, 'product id characters invalid')
    END;
    SELECT CASE
        WHEN NEW.id != lower(trim(NEW.id)) THEN
            RAISE(ABORT, 'product id must be lowercase')
    END;
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'product name required')
    END;
    SELECT CASE
        WHEN NEW.gtin IS NULL OR trim(NEW.gtin) = '' THEN
            RAISE(ABORT, 'product gtin required')
    END;
    SELECT CASE
        WHEN NEW.price_cents <= 0 THEN
            RAISE(ABORT, 'price_cents must be > 0')
    END;
    SELECT CASE
        WHEN NEW.minimum_order_quantity <= 0 THEN
            RAISE(ABORT, 'minimum_order_quantity must be > 0')
    END;
    SELECT CASE
        WHEN NEW.inventory < 0 THEN
            RAISE(ABORT, 'inventory must be >= 0')
    END;
    SELECT CASE
        WHEN NEW.currency NOT IN ('USD', 'EUR', 'GBP') THEN
            RAISE(ABORT, 'currency must be USD, EUR, or GBP')
    END;
    SELECT CASE
        WHEN NEW.product_hash IS NULL OR length(NEW.product_hash) != 32 THEN
            RAISE(ABORT, 'product_hash must be 32 bytes')
    END;
END;

DROP TRIGGER IF EXISTS trg_product_catalog_update_guard;
CREATE TRIGGER trg_product_catalog_update_guard
BEFORE UPDATE OF name, gtin, price_cents, currency, minimum_order_quantity, inventory, is_preorder, estimated_delivery_weeks, supplier_url, image_url, is_active ON product_catalog
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.name IS NULL OR trim(NEW.name) = '' THEN
            RAISE(ABORT, 'product name required')
    END;
    SELECT CASE
        WHEN NEW.gtin IS NULL OR trim(NEW.gtin) = '' THEN
            RAISE(ABORT, 'product gtin required')
    END;
    SELECT CASE
        WHEN NEW.price_cents <= 0 THEN
            RAISE(ABORT, 'price_cents must be > 0')
    END;
    SELECT CASE
        WHEN NEW.minimum_order_quantity <= 0 THEN
            RAISE(ABORT, 'minimum_order_quantity must be > 0')
    END;
    SELECT CASE
        WHEN NEW.inventory < 0 THEN
            RAISE(ABORT, 'inventory must be >= 0')
    END;
    SELECT CASE
        WHEN NEW.currency NOT IN ('USD', 'EUR', 'GBP') THEN
            RAISE(ABORT, 'currency must be USD, EUR, or GBP')
    END;
END;

-- ---------------------------------------------------------------------------
-- user_participation
--
-- Index of confirmed escrow deposits. The on-chain Participation account is
-- the source of truth, but we mirror it here so that "my orders" lookups,
-- "deals with at least one buyer" filters, and per-product participant counts
-- are cheap O(1)/O(log n) D1 queries instead of expensive `getProgramAccounts`
-- RPCs (which most hosted Solana RPCs disable or rate-limit at scale).
--
-- A row is written by the deposit-confirm handler after it verifies the
-- participation PDA exists on-chain. The (uid, product_id) primary key
-- matches the on-chain Participation PDA seeds, so subsequent deposits by
-- the same user for the same product update the same row.
-- ---------------------------------------------------------------------------
CREATE TABLE user_participation (
    uid TEXT NOT NULL,
    product_id TEXT NOT NULL,
    -- The (product_id, batch_id) pair identifies one on-chain Pool. The
    -- same buyer can join successive batches without colliding rows here
    -- because batch_id is part of the primary key.
    batch_id INTEGER NOT NULL,
    wallet_address TEXT NOT NULL,
    product_amount_base_units INTEGER NOT NULL,
    shipping_amount_base_units INTEGER NOT NULL,
    -- Number of product units this buyer committed in the batch. Mirrors
    -- on-chain `Participation.quantity`. Summed across non-refunded rows
    -- to display "X of N units" progress against the batch threshold.
    quantity INTEGER NOT NULL,
    -- Mirrors the on-chain Participation.refunded byte. Set to 1 by the
    -- self-refund-confirm handler after the chain side flips.
    refunded INTEGER NOT NULL DEFAULT 0,
    last_tx_signature TEXT NOT NULL,
    first_deposited_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_deposited_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (uid, product_id, batch_id),
    FOREIGN KEY (product_id) REFERENCES product_catalog(id) ON DELETE CASCADE
) WITHOUT ROWID;

CREATE INDEX idx_user_participation_uid ON user_participation(uid);
CREATE INDEX idx_user_participation_product ON user_participation(product_id);
CREATE INDEX idx_user_participation_product_batch
    ON user_participation(product_id, batch_id);

-- ---------------------------------------------------------------------------
-- product_batch
--
-- Tracks the "current" group-buy batch for each product. Each row mirrors
-- one on-chain Pool keyed by `(product_id, batch_id)`. The cron job adds
-- a new row when the previous batch auto-locks (threshold met) so
-- subsequent deposits target the new pool. The backend reads the
-- highest-batch-id row for a product when answering deposit/intent.
--
-- `pipeline_status` is our internal funnel state, separate from the
-- on-chain `Pool.status`. We can advance through Locked → ShippingToDistributor
-- → ShippingIndividually → Released without any on-chain transition; the
-- on-chain status only flips for actions that require chain-side proof
-- (Locked when threshold hit, Released when funds disbursed, etc.).
-- ---------------------------------------------------------------------------
CREATE TABLE product_batch (
    product_id TEXT NOT NULL,
    batch_id INTEGER NOT NULL,
    threshold INTEGER NOT NULL,
    -- Internal pipeline state. See `BatchPipelineStatus` in
    -- backend-shared/src/route/account/orders.rs for the canonical values.
    pipeline_status TEXT NOT NULL DEFAULT 'open',
    opened_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    locked_at DATETIME,
    closed_at DATETIME,
    PRIMARY KEY (product_id, batch_id),
    FOREIGN KEY (product_id) REFERENCES product_catalog(id) ON DELETE CASCADE
) WITHOUT ROWID;

-- Index used by the deposit/intent path to find the active batch quickly:
-- "the open one with the highest batch_id for this product".
CREATE INDEX idx_product_batch_active
    ON product_batch(product_id, pipeline_status, batch_id);

