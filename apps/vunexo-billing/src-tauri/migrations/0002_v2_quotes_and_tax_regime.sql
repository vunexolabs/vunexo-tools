-- Round 7 (V2): additive migration implementing
-- docs/vunexo-billing/database-schema-v2.md. v1.0.0 is a published release
-- with real installs, so this is a genuine ALTER/CREATE delta, not a
-- replacement of 0001_init.sql — see that document's §11 (Migration strategy).
--
-- Order matters: quotes must exist before invoices.source_quote_id can
-- reference it.

ALTER TABLE business ADD COLUMN tax_regime_code TEXT NOT NULL DEFAULT 'IN_GST'
    CHECK (tax_regime_code IN ('IN_GST', 'VAT_STANDARD'));

ALTER TABLE settings ADD COLUMN quote_number_format TEXT NOT NULL DEFAULT 'QUO-{year}-{seq:04d}';
ALTER TABLE settings ADD COLUMN payment_reminder_template TEXT;

CREATE TABLE quote_number_counters (
    scope_key TEXT PRIMARY KEY,
    last_value INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE quotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    quote_number TEXT,
    status TEXT NOT NULL DEFAULT 'DRAFT'
        CHECK (status IN ('DRAFT', 'ISSUED', 'ACCEPTED', 'DECLINED', 'CONVERTED', 'CANCELLED')),

    customer_id INTEGER REFERENCES customers(id) ON DELETE RESTRICT,

    -- Customer snapshot, frozen at Issue (NULL while DRAFT)
    customer_snapshot_name TEXT,
    customer_snapshot_phone TEXT,
    customer_snapshot_email TEXT,
    customer_snapshot_address TEXT,
    customer_snapshot_gstin TEXT,

    -- Business snapshot, frozen at Issue (NULL while DRAFT)
    business_snapshot_name TEXT,
    business_snapshot_address TEXT,
    business_snapshot_gstin TEXT,
    business_snapshot_phone TEXT,
    business_snapshot_email TEXT,
    business_snapshot_bank_details TEXT,
    business_snapshot_upi_id TEXT,
    business_snapshot_logo_path TEXT,

    tax_regime_snapshot TEXT,
    is_interstate INTEGER NOT NULL DEFAULT 0,

    quote_date TEXT NOT NULL DEFAULT (date('now')),
    valid_until TEXT,

    notes TEXT,
    terms TEXT,

    discount_type TEXT CHECK (discount_type IN ('AMOUNT', 'PERCENTAGE')),
    discount_value INTEGER,

    subtotal_minor INTEGER NOT NULL DEFAULT 0,
    discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    tax_amount_minor INTEGER NOT NULL DEFAULT 0,
    total_minor INTEGER NOT NULL DEFAULT 0,

    issued_at TEXT,
    accepted_at TEXT,
    declined_at TEXT,
    converted_at TEXT,
    cancelled_at TEXT,
    cancel_reason TEXT,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX idx_quotes_number ON quotes(quote_number) WHERE quote_number IS NOT NULL;
CREATE INDEX idx_quotes_customer ON quotes(customer_id);
CREATE INDEX idx_quotes_status ON quotes(status);
CREATE INDEX idx_quotes_quote_date ON quotes(quote_date);
CREATE INDEX idx_quotes_valid_until ON quotes(valid_until);

CREATE TABLE quote_line_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    quote_id INTEGER NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id) ON DELETE RESTRICT,

    -- Frozen at the moment this item is added to the quote
    description TEXT NOT NULL,
    unit TEXT NOT NULL,
    quantity_thousandths INTEGER NOT NULL CHECK (quantity_thousandths > 0),
    unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),

    line_discount_type TEXT CHECK (line_discount_type IN ('AMOUNT', 'PERCENTAGE')),
    line_discount_value INTEGER,

    tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    tax_rate_basis_points INTEGER NOT NULL DEFAULT 0,

    -- Computed and persisted at save time — database-schema-v2.md §4
    line_subtotal_minor INTEGER NOT NULL DEFAULT 0,
    line_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    quote_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    taxable_amount_minor INTEGER NOT NULL DEFAULT 0,
    line_tax_minor INTEGER NOT NULL DEFAULT 0,
    line_total_minor INTEGER NOT NULL DEFAULT 0,

    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_quote_line_items_quote ON quote_line_items(quote_id);
CREATE INDEX idx_quote_line_items_product ON quote_line_items(product_id);

ALTER TABLE invoices ADD COLUMN source_quote_id INTEGER REFERENCES quotes(id) ON DELETE RESTRICT;
ALTER TABLE invoices ADD COLUMN tax_regime_snapshot TEXT;
CREATE UNIQUE INDEX idx_invoices_source_quote ON invoices(source_quote_id) WHERE source_quote_id IS NOT NULL;
