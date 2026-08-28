-- Round 3: core V1 schema. See docs/vunexo-billing/database-schema.md for the
-- full design rationale (snapshot strategy, money/quantity representation,
-- numbering, delete/archive rules) — this file is the executable result of
-- that locked design, not the place to relitigate it.

CREATE TABLE business (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    logo_path TEXT,
    address TEXT,
    phone TEXT,
    email TEXT,
    gstin TEXT,
    bank_details TEXT,
    upi_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tax_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    rate_basis_points INTEGER NOT NULL CHECK (rate_basis_points >= 0),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    country_code TEXT NOT NULL DEFAULT 'IN',
    currency_code TEXT NOT NULL DEFAULT 'INR',
    date_format TEXT NOT NULL DEFAULT 'DD/MM/YYYY',
    invoice_number_format TEXT NOT NULL DEFAULT 'INV-{year}-{seq:04d}',
    default_due_days INTEGER NOT NULL DEFAULT 15,
    default_tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    address TEXT,
    gstin TEXT,
    status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_customers_name ON customers(name);
CREATE INDEX idx_customers_status ON customers(status);

CREATE TABLE products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    sku TEXT,
    description TEXT,
    unit TEXT NOT NULL,
    price_minor INTEGER NOT NULL CHECK (price_minor >= 0),
    tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    hsn_sac_code TEXT,
    status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'ARCHIVED')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_products_name ON products(name);
CREATE INDEX idx_products_status ON products(status);

CREATE TABLE invoice_number_counters (
    scope_key TEXT PRIMARY KEY,
    last_value INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    invoice_number TEXT,
    invoice_number_is_custom INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'DRAFT'
        CHECK (status IN ('DRAFT', 'ISSUED', 'PARTIALLY_PAID', 'PAID', 'CANCELLED')),

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

    is_interstate INTEGER NOT NULL DEFAULT 0,

    invoice_date TEXT NOT NULL DEFAULT (date('now')),
    due_date TEXT,

    notes TEXT,
    terms TEXT,

    discount_type TEXT CHECK (discount_type IN ('AMOUNT', 'PERCENTAGE')),
    discount_value INTEGER,

    subtotal_minor INTEGER NOT NULL DEFAULT 0,
    discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    tax_amount_minor INTEGER NOT NULL DEFAULT 0,
    total_minor INTEGER NOT NULL DEFAULT 0,

    issued_at TEXT,
    cancelled_at TEXT,
    cancel_reason TEXT,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX idx_invoices_number ON invoices(invoice_number) WHERE invoice_number IS NOT NULL;
CREATE INDEX idx_invoices_customer ON invoices(customer_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_invoice_date ON invoices(invoice_date);
CREATE INDEX idx_invoices_due_date ON invoices(due_date);

CREATE TABLE invoice_line_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_id INTEGER REFERENCES products(id) ON DELETE RESTRICT,

    -- Frozen at the moment this item is added to the invoice
    description TEXT NOT NULL,
    unit TEXT NOT NULL,
    quantity_thousandths INTEGER NOT NULL CHECK (quantity_thousandths > 0),
    unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),

    line_discount_type TEXT CHECK (line_discount_type IN ('AMOUNT', 'PERCENTAGE')),
    line_discount_value INTEGER,

    tax_rate_id INTEGER REFERENCES tax_rates(id) ON DELETE SET NULL,
    tax_rate_basis_points INTEGER NOT NULL DEFAULT 0,

    -- Computed and persisted at save time — see §4 (snapshot strategy)
    line_subtotal_minor INTEGER NOT NULL DEFAULT 0,
    line_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    invoice_discount_amount_minor INTEGER NOT NULL DEFAULT 0,
    taxable_amount_minor INTEGER NOT NULL DEFAULT 0,
    line_tax_minor INTEGER NOT NULL DEFAULT 0,
    line_total_minor INTEGER NOT NULL DEFAULT 0,

    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_invoice_line_items_invoice ON invoice_line_items(invoice_id);
CREATE INDEX idx_invoice_line_items_product ON invoice_line_items(product_id);

CREATE TABLE payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    method TEXT NOT NULL CHECK (method IN ('CASH', 'BANK_TRANSFER', 'UPI', 'CHEQUE', 'OTHER')),
    paid_on TEXT NOT NULL,
    reference TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_payments_invoice ON payments(invoice_id);
CREATE INDEX idx_payments_paid_on ON payments(paid_on);
